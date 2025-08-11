use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WpilogReadErrors {
    #[error("file io error")]
    IoError(#[from] io::Error),
    #[error("no data left in file")]
    NoDataLeft,
    #[error("header not valid")]
    InvalidHeader,
    #[error("record {record_num} with the id  {entry_id} is invalid")]
    InvalidRecoard { record_num: u32, entry_id: u32 },
    #[error("wpilog version {major_version}.{minor_version} unsupported", major_version = (version | 0x00ff) / 0xff, minor_version = version | 0xff00)]
    UnsupportedWpilogVersion { version: u16 },
    #[error("data for record {record_num} with entry id {entry_id} is malformed")]
    MalformedData { record_num: u32, entry_id: u32 },
    #[error("record {record_num} used entry id {entry_id} before a start entry")]
    UseOfEntryIdWithoutStart { record_num: u32, entry_id: u32 },
    #[error("record {record_num} used {entry_id} after a finish entry")]
    UseOfEntryIdAfterFinish { record_num: u32, entry_id: u32 },
    #[error("record {record_num} tried to start entry id {entry_id} after it was already started")]
    EntryIdAlreadyStarted { record_num: u32, entry_id: u32 },
    #[error("record {record_num} tried to set metadata of entry {entry_id} before a start entry")]
    SetMetadataWithoutStart { record_num: u32, entry_id: u32 },
    #[error("record {record_num} tried to finish entry {entry_id} before a start entry")]
    FinishWithoutStart { record_num: u32, entry_id: u32 },
}
pub fn pad_to_n_bytes<const SIZE: usize>(data: Vec<u8>) -> [u8; SIZE] {
    let mut arr: [u8; SIZE] = [0; SIZE];
    for (i, byte) in arr.iter_mut().enumerate() {
        *byte = match data.get(i) {
            Some(n) => *n,
            None => 0,
        };
    }
    arr
}

pub fn next_chunk<const SIZE: usize>(
    file: &mut (Vec<u8>, usize),
) -> Result<[u8; SIZE], WpilogReadErrors> {
    let mut out = [0; SIZE];
    let mut iter = file.0.iter().skip(file.1);
    for byte in out.iter_mut() {
        *byte = match iter.next() {
            Some(n) => *n,
            None => return Err(WpilogReadErrors::NoDataLeft),
        };
    }
    file.1 += SIZE;

    Ok(out)
}
pub fn next_chunk_vec(
    file: &mut (Vec<u8>, usize),
    size: usize,
) -> Result<Vec<u8>, WpilogReadErrors> {
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

    Ok(out)
}
