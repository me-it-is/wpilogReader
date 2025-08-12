#![warn(clippy::all)]
use std::{
    collections::HashMap,
    fs::{File, read_dir},
    io::Read,
};

mod control_records;
mod headers;
mod records;
mod shared;
use headers::{FileHeader, read_header};
use records::{Entry, Record, read_next_record};
use shared::WpilogReadErrors;

#[allow(dead_code)]
struct Wpilog<'a> {
    header: FileHeader,
    records: Vec<&'a Record>,
    entry_lut: HashMap<u32, Entry<'a>>,
}

fn read_wpilog<'a>(path: &'a str) -> Result<Wpilog<'_>, WpilogReadErrors> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(err) => return Err(WpilogReadErrors::IoError(err)),
    };
    let mut file_content = Vec::new();
    match file.read_to_end(&mut file_content) {
        Ok(n) => _ = n,
        Err(err) => return Err(WpilogReadErrors::IoError(err)),
    };

    let mut entry_lut: HashMap<u32, Entry> = HashMap::new();
    let mut file_to_read: (Vec<u8>, usize) = (file_content, 0);
    let header = read_header(&mut file_to_read)?;

    let mut records: Vec<&'a Record> = Vec::new();

    loop {
        let record = match read_next_record(&mut file_to_read, &mut entry_lut, records.len() as u32)
        {
            Ok(r) => r,
            Err(e) => match e {
                WpilogReadErrors::NoDataLeft => break,
                _ => return Err(e),
            },
        };
        records.push(&record);
    }

    Ok(Wpilog {
        header,
        records,
        entry_lut,
    })
}

fn main() {
    let mut wpilog_path = dirs::home_dir().unwrap();
    wpilog_path.push("Documents/code/robotics/wpilogReader/tests");
    let paths = read_dir(wpilog_path).unwrap();
    for path in paths {
        let raw_path_str = path.unwrap().path();
        let path_str = raw_path_str.to_str().unwrap();
        if path_str.ends_with(".wpilog") {
            read_wpilog(path_str).unwrap();
        }
    }
}
