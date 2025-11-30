use crate::shared::{WpilogReadErrors, next_chunk, next_chunk_slice, no_data_err_if_none};

pub struct FileHeader {
    pub version_number: u16,
    pub extra_string: String,
}

pub fn read_header(file: &mut &[u8]) -> Result<FileHeader, WpilogReadErrors> {
    if (match str::from_utf8(no_data_err_if_none!(next_chunk_slice(file, 6))) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidHeader),
    } != "WPILOG")
    {
        return Err(WpilogReadErrors::InvalidHeader);
    }
    let raw_version_number = no_data_err_if_none!(next_chunk(file));
    let version_number = u16::from_le_bytes(raw_version_number);

    if version_number != 0x0100 {
        return Err(WpilogReadErrors::UnsupportedWpilogVersion {
            version: version_number,
        });
    }

    let extra_string_length = u32::from_le_bytes(no_data_err_if_none!(next_chunk(file)));

    let extra_string = no_data_err_if_none!(next_chunk_slice(file, extra_string_length as usize));
    let extra_string = match str::from_utf8(extra_string) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidHeader),
    }
    .to_string();

    Ok(FileHeader {
        version_number,
        extra_string,
    })
}
