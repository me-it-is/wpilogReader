use thiserror::Error;

macro_rules! no_data_err_if_none {
    ($val:expr) => {
        match $val {
            None => return Err(WpilogReadErrors::NoDataLeft),
            Some(d) => d,
        }
    };
}

pub(crate) use no_data_err_if_none;

#[derive(Debug, Error)]
pub enum WpilogReadErrors {
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

#[derive(Debug, Error)]
pub enum WpilogEncodeErrors {
    #[error("cant encode a record")]
    CantEncodeRecord,
    #[error("wpilog version {major_version}.{minor_version} unsupported", major_version = (version | 0x00ff) / 0xff, minor_version = version | 0xff00)]
    UnsupportedWpilogVersion { version: u16 },
}
pub fn pad_to_n_bytes<const SIZE: usize>(data: &[u8]) -> [u8; SIZE] {
    let mut arr: [u8; SIZE] = [0; SIZE];
    for (i, byte) in arr.iter_mut().enumerate() {
        *byte = match data.get(i) {
            Some(n) => *n,
            None => 0,
        };
    }
    arr
}

pub fn next_chunk<const SIZE: usize>(file: &mut &[u8]) -> Option<[u8; SIZE]> {
    let split = file.split_first_chunk();
    let (out, remaining_file) = split?;
    *file = remaining_file;
    Some(*out)
}

pub fn next_chunk_slice<'a>(file: &mut &'a [u8], size: usize) -> Option<&'a [u8]> {
    let split = file.split_at_checked(size);
    let (out, remaining_file) = split?;
    *file = remaining_file;
    Some(out)
}

pub fn bool_to_byte(bool: &bool) -> u8 {
    match bool {
        true => 1,
        false => 0,
    }
}
