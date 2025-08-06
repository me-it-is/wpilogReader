use std::{collections::HashMap, fs::File, io::Read, usize};

mod headers;
mod shared;
mod records;
mod control_records;
use headers::headers::{FileHeader, read_header};
use shared::shared::{WpilogReadErrors};
use records::records::{Record, EntryData, read_next_record};

static WPILOG_PATH: &str = "test.wpilog";
#[allow(dead_code)]
struct Wpilog {
    header: FileHeader,
    records: Vec<Record>,
    entry_data_lut: HashMap<u32, Vec<EntryData>>,
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

fn main() {
    read_wpilog(WPILOG_PATH).unwrap();
}
