#![warn(clippy::all)]
use std::{
    collections::HashMap,
    fs::{File, read_dir},
    io::Read,
};

mod control_records;
mod encode_records;
mod headers;
mod records;
mod shared;
use encode_records::record_to_bytes;
use headers::{FileHeader, read_header};
use records::{Entry, read_next_record};
use shared::WpilogReadErrors;

#[allow(dead_code)]
struct Wpilog<'a> {
    header: FileHeader,
    entry_lut: HashMap<u32, Entry<'a>>,
}

fn read_wpilog<'a>(data: &'a [u8]) -> Result<Wpilog<'a>, WpilogReadErrors> {
    let mut entry_lut: HashMap<u32, Entry<'a>> = HashMap::new();
    let mut file_to_read: &'a [u8] = data;
    let header = read_header(&mut file_to_read)?;

    let mut record_num = 0;
    loop {
        match read_next_record(&mut file_to_read, &mut entry_lut, record_num as u32) {
            Ok(r) => r,
            Err(WpilogReadErrors::NoDataLeft) => break,
            Err(e) => return Err(e),
        };
        record_num += 1;
    }

    Ok(Wpilog {
        header,
        entry_lut: entry_lut,
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
            let mut file = File::open(path_str).unwrap();
            let mut file_content = Vec::new();
            file.read_to_end(&mut file_content).unwrap();
            let wpilog = read_wpilog(file_content.as_slice()).unwrap();
            println!("{}", path_str);
            for (_, entry) in wpilog.entry_lut {
                //println!("metadata:{:?}", entry.meta_data);
                for record in &entry.records {
                    _ = record_to_bytes(&record);
                }
            }
        }
    }
}
