use crate::{headers::FileHeader, shared::WpilogEncodeErrors};

pub fn encode_header(header: &FileHeader) -> Result<Vec<u8>, WpilogEncodeErrors> {
    let mut out = vec![];

    out.extend_from_slice("WPILOG".as_bytes());

    if header.version_number != 0x0100 {
        return Err(WpilogEncodeErrors::UnsupportedWpilogVersion {
            version: header.version_number,
        });
    }
    out.extend_from_slice(&u16::to_le_bytes(header.version_number));

    out.extend_from_slice(&u32::to_le_bytes(header.extra_string.len() as u32));
    out.extend_from_slice(header.extra_string.as_bytes());

    Ok(out)
}
