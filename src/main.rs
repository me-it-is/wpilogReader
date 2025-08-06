#![warn(clippy::all)]
use std::{collections::HashMap, fs::File, io::Read, usize};

mod control_records;
mod headers;
mod records;
mod shared;
use headers::headers::{FileHeader, read_header};
use records::records::{Entry, Record, read_next_record};
use shared::shared::WpilogReadErrors;

static WPILOG_PATH: &str = "test.wpilog";
#[allow(dead_code)]
struct Wpilog<'a> {
    header: FileHeader,
    records: Vec<Record>,
    entry_lut: HashMap<u32, Entry<'a>>,
}

fn read_wpilog(path: &str) -> Result<Wpilog<'_>, WpilogReadErrors> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Err(WpilogReadErrors::FileDoesNotExist),
    };
    let mut file_content = Vec::new();
    match file.read_to_end(&mut file_content) {
        Ok(n) => _ = n,
        Err(_) => return Err(WpilogReadErrors::ReadError),
    };

    let mut entry_lut: HashMap<u32, Entry> = HashMap::new();
    let mut file_to_read: (Vec<u8>, usize) = (file_content, 0);
    let header = read_header(&mut file_to_read)?;

    let mut records: Vec<Record> = Vec::new();

    loop {
        match read_next_record(&mut file_to_read, &mut entry_lut, records.len() as u32) {
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
        entry_lut: entry_lut,
    });
}

fn main() {
    read_wpilog(WPILOG_PATH).unwrap();
}
