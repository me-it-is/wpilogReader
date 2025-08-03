use core::net;
use std::{fs::File, io::Read, time::Duration, usize};

static WPILOG_PATH: &str = "test.wpilog";

struct Wpilog {
    header: FileHeader,
    records: Vec<Record>,
}

#[derive(Debug, Clone)]
enum WpilogReadErrors {
    FileDoesNotExist,
    ReadError,
    NoDataLeft,
    InvalidHeader,
    InvalidRecoard,
}

struct Record {
    entry_id: u32,
    time_stamp: Duration,
    data: RecordData,
}

enum RecordData {
    Start(StartRecordData),
    Finish(FinishRecordData),
    SetMetadata(SetMetaDataRecordData),
    NormalData(NormalRecordData),
}

enum ControlTypes {
    Start,
    Finish,
    SetMetadata,
}

struct StartRecordData {
    entry_id_to_be_started: u32,
    entry_name: String,
    entry_type: String,
    entry_metadata: String,
}

struct FinishRecordData {
    entry_to_be_finished: u32,
}

struct SetMetaDataRecordData {
    entry_to_be_edited: u32,
    entry_new_metadata: String,
}

struct NormalRecordData {
    data: Vec<u8>,
}

struct FileHeader {
    version_number: u16,
    extra_string: String,
}

fn read_wpilog(path: &str) -> Result<Wpilog, WpilogReadErrors> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Err(WpilogReadErrors::FileDoesNotExist),
    };
    let mut file_content = Vec::new();
    match file.read_to_end(&mut file_content) {
        Ok(n) => _ = n,
        Err(_) => return Err(WpilogReadErrors::ReadError),
    };
    println!("{}", file_content.len());

    let mut file_to_read: (Vec<u8>, usize) = (file_content, 0);
    let header = read_header(&mut file_to_read)?;

    let records = read_next_record(&mut file_to_read)?;

    return Ok(Wpilog {
        header: header,
        records: vec![records],
    });
}

fn read_header(file: &mut (Vec<u8>, usize)) -> Result<FileHeader, WpilogReadErrors> {
    if (match str::from_utf8(next_chunk(file, 6)?.as_slice()) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidHeader),
    } != "WPILOG")
    {
        return Err(WpilogReadErrors::InvalidHeader);
    }

    let version_number = u16::from_le_bytes(next_chunk(file, 2)?.try_into().unwrap());

    let extra_string_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());

    let extra_string_raw = next_chunk(file, extra_string_length as usize)?;
    let extra_string = match str::from_utf8(extra_string_raw.as_slice()) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidHeader),
    };

    return Ok(FileHeader {
        version_number: version_number,
        extra_string: extra_string.to_string(),
    });
}

fn read_next_record(file: &mut (Vec<u8>, usize)) -> Result<Record, WpilogReadErrors> {
    let header_bit_field = next_chunk(file, 1)?[0];

    let entry_id = u32::from_le_bytes(pad_to_n_bytes(next_chunk(
        file,
        ((header_bit_field & 0b11000000) + 1) as usize,
    )?));

    let payload_size = u32::from_le_bytes(pad_to_n_bytes(next_chunk(
        file,
        ((header_bit_field & 0b00110000) + 1) as usize,
    )?));

    let timestamp = u64::from_le_bytes(pad_to_n_bytes(next_chunk(
        file,
        ((header_bit_field & 0b00001110) + 1) as usize,
    )?));

    let data: RecordData;

    if entry_id == 0 {
        data = process_control_recoard(file)?;
    } else {
        data = RecordData::NormalData(NormalRecordData {
            data: next_chunk(file, payload_size as usize)?,
        });
    }

    return Ok(Record {
        entry_id: entry_id as u32,
        time_stamp: Duration::from_micros(timestamp),
        data: data,
    });
}

fn process_control_recoard(file: &mut (Vec<u8>, usize)) -> Result<RecordData, WpilogReadErrors> {
    let control_type = match next_chunk(file, 1)?[0] {
        0 => ControlTypes::Start,
        1 => ControlTypes::Finish,
        2 => ControlTypes::SetMetadata,
        _ => return Err(WpilogReadErrors::InvalidRecoard),
    };

    return match control_type {
        ControlTypes::Start => process_start_recoard(file),
        ControlTypes::Finish => process_finish_recoard(file),
        ControlTypes::SetMetadata => process_set_metadata_recoard(file),
    };
}

fn process_start_recoard(file: &mut (Vec<u8>, usize)) -> Result<RecordData, WpilogReadErrors> {
    let entry_id_to_be_started = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());

    let entry_name_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());
    let entry_name_string_raw = next_chunk(file, entry_name_length as usize)?;
    let entry_name_string = match str::from_utf8(entry_name_string_raw.as_slice()) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidRecoard),
    };

    let entry_type_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());
    let entry_type_string_raw = next_chunk(file, entry_type_length as usize)?;
    let entry_type_string = match str::from_utf8(entry_type_string_raw.as_slice()) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidRecoard),
    };

    let entry_metadata_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());
    let entry_metadata_string_raw = next_chunk(file, entry_metadata_length as usize)?;
    let entry_metadata_string = match str::from_utf8(entry_metadata_string_raw.as_slice()) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidRecoard),
    };

    return Ok(RecordData::Start(StartRecordData {
        entry_id_to_be_started,
        entry_name: entry_name_string.to_string(),
        entry_type: entry_type_string.to_string(),
        entry_metadata: entry_metadata_string.to_string(),
    }));
}

fn process_finish_recoard(file: &mut (Vec<u8>, usize)) -> Result<RecordData, WpilogReadErrors> {
    let entry_id_to_be_finished = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());

    return Ok(RecordData::Finish(FinishRecordData {
        entry_to_be_finished: entry_id_to_be_finished,
    }));
}
fn process_set_metadata_recoard(
    file: &mut (Vec<u8>, usize),
) -> Result<RecordData, WpilogReadErrors> {
    let entry_id_to_set_metadata = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());

    let entry_metadata_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());
    let entry_metadata_string_raw = next_chunk(file, entry_metadata_length as usize)?;
    let entry_metadata_string = match str::from_utf8(entry_metadata_string_raw.as_slice()) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidRecoard),
    };

    return Ok(RecordData::SetMetadata(SetMetaDataRecordData {
        entry_to_be_edited: entry_id_to_set_metadata,
        entry_new_metadata: entry_metadata_string.to_string(),
    }));
}
fn pad_to_n_bytes<const SIZE: usize>(data: Vec<u8>) -> [u8; SIZE] {
    let mut arr: [u8; SIZE] = [0; SIZE];
    for i in 0..SIZE {
        arr[i] = match data.get(i) {
            Some(n) => *n,
            None => 0,
        };
    }
    return arr;
}

fn next_chunk(file: &mut (Vec<u8>, usize), size: usize) -> Result<Vec<u8>, WpilogReadErrors> {
    let mut out: Vec<u8> = Vec::new();
    let mut iter = file.0.iter().skip(file.1);
    for _i in 0..size {
        out.push(
            *(match iter.next() {
                Some(n) => n,
                None => return Err(WpilogReadErrors::NoDataLeft),
            }),
        );
    }
    file.1 += size;

    return Ok(out);
}

fn main() {
    read_wpilog(WPILOG_PATH).unwrap();
}
