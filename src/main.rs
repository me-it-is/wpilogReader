use std::{fs::{File}, io::{Read}};

static WPILOG_PATH: &str = "test.wpilog";

struct Wpilog {
    header: FileHeader
}

#[derive(Debug, Clone)]
enum WpilogReadErrors {
    FileDoesNotExist,
    ReadError,
    NoDataLeft,
    InvalidHeader
}

struct Record {
    entry_id: u32,
    time_stamp: u64,
    data: Vec<u8>
}

struct FileHeader {
    version_number: u16,
    extra_string: String
}

fn read_wpilog(path: &str) -> Result<Wpilog, WpilogReadErrors> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Err(WpilogReadErrors::FileDoesNotExist),
    };
    let mut file_content= Vec::new();
    match file.read_to_end(&mut file_content) {Ok(n) => _ = n, Err(_) => return Err(WpilogReadErrors::ReadError)};
    let header = read_header(&mut (file_content, 0))?;

    return Ok(Wpilog { header: header });
}

fn read_header(file: &mut (Vec<u8>, usize)) -> Result<FileHeader, WpilogReadErrors> {
    if (match str   ::from_utf8(match next_chunk(file, 6) {Ok(c) => c, Err(_) => return Err(WpilogReadErrors::ReadError)}.as_slice()) {Ok(s) => s, Err(_) => return Err(WpilogReadErrors::ReadError)} != "WPILOG") {
        return Err(WpilogReadErrors::InvalidHeader);
    }

    let version_number = u16::from_le_bytes(match next_chunk(file, 2) {Ok(c) => c, Err(_) => return Err(WpilogReadErrors::InvalidHeader)}.try_into().unwrap());

    let extra_string_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());

    let extra_string_raw = next_chunk(file, extra_string_length as usize)?;
    let extra_string = match str::from_utf8(extra_string_raw.as_slice()) {Ok(s) => s, Err(_) => return  Err(WpilogReadErrors::InvalidHeader)};
    
    return Ok(FileHeader { version_number: version_number, extra_string: extra_string.to_string()})
}

fn next_chunk(file: &mut (Vec<u8>, usize),size: usize) -> Vec<u8> {
    let mut out: Vec<u8> =  Vec::new();
    let mut iter = file.0.iter().skip(file.1);
    for _i in 0..size {
        out.push(*(match iter.next() {Some(n) => n, None => return Err(WpilogReadErrors::NoDataLeft)}));
    } 
    file.1 += size;

    return Ok(out);
}


fn main() {
    read_wpilog(WPILOG_PATH).unwrap();
}
