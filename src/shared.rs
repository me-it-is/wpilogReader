use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum WpilogReadErrors {
    #[error("wpilog does not exist")]
    FileDoesNotExist,
    #[error("cant read file")]
    ReadError,
    #[error("no data left in file")]
    NoDataLeft,
    #[error("header not valid")]
    InvalidHeader,
    #[error("invalid record")]
    InvalidRecoard,
    #[error("wpilog version unsupported")]
    UnsupportedWpilogVersion,
    #[error("data is malformed")]
    MalformedData,
    #[error("record id used before a start entry")]
    UseOfEntryIdWithoutStart,
    #[error("recprd id used after a finish entry")]
    UseOfEntryIdAfterFinish,
    #[error("entry id already started")]
    EntryIdAlreadyStarted,
    #[error("cannot set metadat for entry without start")]
    SetMetadataWithoutStart,
    #[error("finish entry without a start entry")]
    FinishWithoutStart,
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

pub fn next_chunk(file: &mut (Vec<u8>, usize), size: usize) -> Result<Vec<u8>, WpilogReadErrors> {
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
