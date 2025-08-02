use std::{fs::{File}, io::{Read}};

static WPILOG_PATH: &str = "test.wpilog";

struct Wpilog {
    header: Header
}

#[derive(Debug, Clone)]
enum WpilogReadErrors {
    FileDoesNotExist,
    ReadError
}

struct Record {
    entry_id: u32,
    time_stamp: u64,
    data: Vec<u8>
}

struct Header {
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

fn read_header(file: &mut (Vec<u8>, usize)) -> Result<Header, WpilogReadErrors> {
    assert!(match str::from_utf8(next_chunk(file, 6).as_slice()) {Ok(s) => s, Err(_) => return Err(WpilogReadErrors::ReadError)} == "WPILOG");

    let version_number = u16::from_le_bytes(next_chunk(file, 2).try_into().unwrap());

    let extra_string_length = u32::from_le_bytes(next_chunk(file, 4).try_into().unwrap());

    let extra_string_raw = next_chunk(file, extra_string_length as usize);
    let extra_string = match str::from_utf8(extra_string_raw.as_slice()) {Ok(s) => s, Err(_) => return  Err(WpilogReadErrors::ReadError)};
    
    return Ok(Header { version_number: version_number, extra_string: extra_string.to_string()})
}

fn next_chunk(file: &mut (Vec<u8>, usize),size: usize) -> Vec<u8> {
    let mut out: Vec<u8> =  Vec::new();
    let mut iter = file.0.iter().skip(file.1);
    for _i in 0..size {
        out.push(*iter.next().unwrap());
    } 
    file.1 += size;

    return out;
}


fn main() {
    read_wpilog(WPILOG_PATH).unwrap();
}
