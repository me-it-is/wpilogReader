pub mod headers {
    use crate::shared::*;
    use shared::WpilogReadErrors;
    use shared::next_chunk;

    #[allow(dead_code)]
    pub struct FileHeader {
        version_number: u16,
        extra_string: String,
    }

    pub fn read_header(file: &mut (Vec<u8>, usize)) -> Result<FileHeader, WpilogReadErrors> {
        if (match str::from_utf8(next_chunk(file, 6)?.as_slice()) {
            Ok(s) => s,
            Err(_) => return Err(WpilogReadErrors::InvalidHeader),
        } != "WPILOG")
        {
            return Err(WpilogReadErrors::InvalidHeader);
        }
        let raw_version_number = next_chunk(file, 2)?.try_into().unwrap();
        let version_number = u16::from_le_bytes(raw_version_number);

        if version_number != 0x0100 {
            return Err(WpilogReadErrors::UnsupportedWpilogVersion);
        }

        let extra_string_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());

        let extra_string_raw = next_chunk(file, extra_string_length as usize)?;
        let extra_string = match str::from_utf8(extra_string_raw.as_slice()) {
            Ok(s) => s,
            Err(_) => return Err(WpilogReadErrors::InvalidHeader),
        };

        return Ok(FileHeader {
            version_number: version_number,
            extra_string: extra_string.to_string(),
        });
    }
}
