use std::{collections::HashMap, fs::File, io::Read, time::Duration, usize};

static WPILOG_PATH: &str = "test.wpilog";
#[allow(dead_code)]
struct Wpilog {
    header: FileHeader,
    records: Vec<Record>,
    entry_data_lut: HashMap<u32, Vec<EntryData>>,
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct EntryData {
    start_record_index: u32,
    name: String,
    data_type: String,
    metadata: String,
    finish_record_index: Option<u32>,
}

#[derive(Debug, Clone)]
enum WpilogReadErrors {
    FileDoesNotExist,
    ReadError,
    NoDataLeft,
    InvalidHeader,
    InvalidRecoard,
    UnsupportedWpilogVersion,
    UnknownDataType,
    MalformedData,
    UseOfEntryIdWithoutStart,
    UseOfEntryIdAfterFinish,
    EntryIdAlreadyStarted,
}
#[allow(dead_code)]
struct Record {
    entry_id: u32,
    time_stamp: Duration,
    data: RecordData,
}
#[allow(dead_code)]
enum RecordData {
    Start(StartRecordData),
    Finish(FinishRecordData),
    SetMetadata(SetMetaDataRecordData),
    Raw(Vec<u8>),
    Boolean(bool),
    Integer(i64),
    Float(f32),
    Double(f64),
    String(String),
    BooleanArray(Vec<bool>),
    IntegerArray(Vec<i64>),
    FloatArray(Vec<f32>),
    DoubleArray(Vec<f64>),
    StringArray(Vec<String>),
    Json(Option<serde_json::Value>),
}

enum ControlTypes {
    Start,
    Finish,
    SetMetadata,
}
#[allow(dead_code)]
struct StartRecordData {
    entry_id_to_be_started: u32,
    entry_name: String,
    entry_type: String,
    entry_metadata: String,
}
#[allow(dead_code)]
struct FinishRecordData {
    entry_to_be_finished: u32,
}
#[allow(dead_code)]
struct SetMetaDataRecordData {
    entry_to_be_edited: u32,
    entry_new_metadata: String,
}
#[allow(dead_code)]
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

    let mut entry_data_lut: HashMap<u32, Vec<EntryData>> = HashMap::new();
    let mut file_to_read: (Vec<u8>, usize) = (file_content, 0);
    let header = read_header(&mut file_to_read)?;

    let mut records: Vec<Record> = Vec::new();

    loop {
        match read_next_record(&mut file_to_read, &mut entry_data_lut, records.len() as u32) {
            Ok(r) => records.push(r),
            Err(e) => match e {
                WpilogReadErrors::NoDataLeft => break,
                _ => return Err(e),
            },
        }
    }

    return Ok(Wpilog {
        header: header,
        records: records,
        entry_data_lut: entry_data_lut,
    });
}

fn get_current_entry_data(
    entry_data_lut: &HashMap<u32, Vec<EntryData>>,
    current_record: u32,
    entry_id: u32,
) -> Result<&EntryData, WpilogReadErrors> {
    let current_id_data = match entry_data_lut.get(&entry_id) {
        Some(l) => l,
        None => return Err(WpilogReadErrors::UseOfEntryIdWithoutStart),
    };

    for data in current_id_data {
        match data.finish_record_index {
            None => return Ok(data),
            Some(i) => {
                if i < current_record {
                    return Ok(data);
                }
            }
        }
    }

    return Err(WpilogReadErrors::UseOfEntryIdAfterFinish);
}

fn read_header(file: &mut (Vec<u8>, usize)) -> Result<FileHeader, WpilogReadErrors> {
    if (match str::from_utf8(next_chunk(file, 6)?.as_slice()) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidHeader),
    } != "WPILOG")
    {
        return Err(WpilogReadErrors::InvalidHeader);
    }
    let raw_version_number = next_chunk(file, 2)?.try_into().unwrap();
    let version_number = u16::from_le_bytes(raw_version_number);

    if version_number != 0x0100 {
        return Err(WpilogReadErrors::UnsupportedWpilogVersion);
    }

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

fn read_next_record(
    file: &mut (Vec<u8>, usize),
    entry_data_lut: &mut HashMap<u32, Vec<EntryData>>,
    current_record: u32,
) -> Result<Record, WpilogReadErrors> {
    let header_bit_field = next_chunk(file, 1)?[0];

    let entry_id = u32::from_le_bytes(pad_to_n_bytes(next_chunk(
        file,
        ((header_bit_field & 0b000011) + 1) as usize,
    )?));

    let raw_payload_size = pad_to_n_bytes(next_chunk(
        file,
        (((header_bit_field & 0b00001100) >> 2) + 1) as usize,
    )?);
    let payload_size = u32::from_le_bytes(raw_payload_size);

    let timestamp = u64::from_le_bytes(pad_to_n_bytes(next_chunk(
        file,
        (((header_bit_field & 0b01110000) >> 4) + 1) as usize,
    )?));

    let data;

    if entry_id == 0 {
        data = process_control_record(file, current_record, entry_data_lut)?;
    } else {
        let raw_data = next_chunk(file, payload_size as usize)?;
        data = process_data_from_standard_record(
            &get_current_entry_data(entry_data_lut, current_record, entry_id)?.data_type,
            raw_data,
        )?;
    }

    return Ok(Record {
        entry_id: entry_id as u32,
        time_stamp: Duration::from_micros(timestamp),
        data: data,
    });
}

fn process_boolean(byte: [u8; 1]) -> Result<bool, WpilogReadErrors> {
    return match byte[0] {
        0 => Ok(false),
        1 => Ok(true),
        _ => return Err(WpilogReadErrors::MalformedData),
    };
}

fn process_data_from_standard_record(
    data_type: &String,
    data: Vec<u8>,
) -> Result<RecordData, WpilogReadErrors> {
    return Ok(match data_type.as_str() {
        "raw" => RecordData::Raw(data),
        "boolean" => RecordData::Boolean(process_boolean(data.try_into().unwrap())?),
        "int64" => RecordData::Integer(i64::from_le_bytes(data.try_into().unwrap())),
        "float" => RecordData::Float(f32::from_le_bytes(data.try_into().unwrap())),
        "double" => RecordData::Double(f64::from_le_bytes(data.try_into().unwrap())),
        "string" => RecordData::String(match String::from_utf8(data) {
            Ok(s) => s,
            Err(_) => return Err(WpilogReadErrors::MalformedData),
        }),
        "boolean[]" => RecordData::BooleanArray(process_array_data(data, &(process_boolean))?),
        "int64[]" => {
            RecordData::IntegerArray(process_array_data_no_err(data, &i64::from_le_bytes)?)
        }
        "float[]" => RecordData::FloatArray(process_array_data_no_err(data, &f32::from_le_bytes)?),
        "double[]" => {
            RecordData::DoubleArray(process_array_data_no_err(data, &f64::from_le_bytes)?)
        }
        "string[]" => match process_string_array(data) {
            Ok(sa) => sa,
            Err(_) => return Err(WpilogReadErrors::MalformedData),
        },
        "json" => RecordData::Json(match data.len() {
            0 => None,
            _ => match serde_json::from_slice(&data) {
                Ok(j) => j,
                Err(_) => return Err(WpilogReadErrors::MalformedData),
            },
        }),
        str => {
            //return Err(WpilogReadErrors::UnknownDataType);
            RecordData::Raw(data)
        }
    });
}

fn process_array_data_no_err<T, const DATA_SIZE: usize>(
    data: Vec<u8>,
    from_func: &dyn Fn([u8; DATA_SIZE]) -> T,
) -> Result<Vec<T>, WpilogReadErrors> {
    return process_array_data(data, &|data: [u8; DATA_SIZE]| Ok(from_func(data)));
}

fn process_array_data<T, const DATA_SIZE: usize>(
    data: Vec<u8>,
    from_func: &dyn Fn([u8; DATA_SIZE]) -> Result<T, WpilogReadErrors>,
) -> Result<Vec<T>, WpilogReadErrors> {
    let mut out = Vec::new();
    let mut entries = data.chunks_exact(DATA_SIZE);

    loop {
        match entries.next() {
            Some(e) => out.push(from_func(e.try_into().unwrap())?),
            None => break,
        }
    }
    if entries.remainder().len() != 0 {
        return Err(WpilogReadErrors::MalformedData);
    }
    return Ok(out);
}

fn process_string_array(data: Vec<u8>) -> Result<RecordData, WpilogReadErrors> {
    let mut indexer = (data, 0);
    let length = u32::from_le_bytes(next_chunk(&mut indexer, 4)?.try_into().unwrap());
    let mut out = Vec::new();

    for _i in 0..length {
        let string_length = u32::from_le_bytes(next_chunk(&mut indexer, 4)?.try_into().unwrap());
        out.push(
            match String::from_utf8(next_chunk(&mut indexer, string_length as usize)?) {
                Ok(s) => s,
                Err(_) => return Err(WpilogReadErrors::MalformedData),
            },
        );
    }

    return Ok(RecordData::StringArray(out));
}

fn process_control_record(
    file: &mut (Vec<u8>, usize),
    current_record: u32,
    entry_data_lut: &mut HashMap<u32, Vec<EntryData>>,
) -> Result<RecordData, WpilogReadErrors> {
    let raw_control_type = next_chunk(file, 1)?[0];
    let control_type = match raw_control_type {
        0 => ControlTypes::Start,
        1 => ControlTypes::Finish,
        2 => ControlTypes::SetMetadata,
        _ => return Err(WpilogReadErrors::InvalidRecoard),
    };

    return match control_type {
        ControlTypes::Start => process_start_recoard(file, current_record, entry_data_lut),
        ControlTypes::Finish => process_finish_recoard(file),
        ControlTypes::SetMetadata => process_set_metadata_recoard(file),
    };
}

fn process_start_recoard(
    file: &mut (Vec<u8>, usize),
    current_record: u32,
    entry_data_lut: &mut HashMap<u32, Vec<EntryData>>,
) -> Result<RecordData, WpilogReadErrors> {
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

    let entry_data = EntryData {
        start_record_index: current_record,
        data_type: entry_type_string.to_string(),
        name: entry_name_string.to_string(),
        metadata: entry_metadata_string.to_string(),
        finish_record_index: None,
    };

    match entry_data_lut.get_mut(&entry_id_to_be_started) {
        None => _ = entry_data_lut.insert(entry_id_to_be_started, vec![entry_data]),
        Some(current) => {
            let last_index = current.len() - 1;
            if current[last_index].finish_record_index.is_none() {
                return Err(WpilogReadErrors::EntryIdAlreadyStarted);
            }
            current[last_index] = entry_data
        }
    }

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
