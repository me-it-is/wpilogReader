use crate::{records::Record, shared::WpilogReadErrors};

pub fn record_to_bytes(record: &Record) -> Result<Vec<u8>, WpilogReadErrors> {
    let mut out: Vec<u8> = vec![];
    let mut payload = record.data.to_bytes()?;
    let payload_size = payload.len() as u32;
    let bit_field_lengths = get_header_bit_field_lengths(record, &payload_size)?;

    out.push(convert_bit_field_lengths_to_bit_fields(&bit_field_lengths));

    out.append(&mut read_n_bytes_of_val(
        record.entry_id as u64,
        bit_field_lengths.entry_id_length,
    ));
    out.append(&mut read_n_bytes_of_val(
        payload_size as u64,
        bit_field_lengths.payload_length,
    ));
    out.append(&mut read_n_bytes_of_val(
        record.timestamp.as_micros() as u64,
        bit_field_lengths.timestamp_length,
    ));
    out.append(&mut payload);

    Ok(out)
}

fn read_n_bytes_of_val(val: u64, n: u8) -> Vec<u8> {
    let mut out = vec![];
    out.reserve_exact(n as usize);

    for i in 0..n {
        out.push(((val | (0xff00000000000000 >> (i * 8))) << (i * 8)) as u8)
    }

    out
}

struct BitFieldLengths {
    entry_id_length: u8,
    payload_length: u8,
    timestamp_length: u8,
}

fn get_header_bit_field_lengths(
    record: &Record,
    data_size: &u32,
) -> Result<BitFieldLengths, WpilogReadErrors> {
    let entry_id_length = size_to_num_bytes(record.entry_id as u64);
    let payload_size = data_size;
    let payload_length = size_to_num_bytes(*payload_size as u64);
    let timestamp_length = size_to_num_bytes(record.timestamp.as_micros() as u64);

    Ok(BitFieldLengths {
        entry_id_length,
        payload_length,
        timestamp_length,
    })
}

fn convert_bit_field_lengths_to_bit_fields(bit_field_lengths: &BitFieldLengths) -> u8 {
    let mut bit_field = 0;
    bit_field |= (bit_field_lengths.entry_id_length - 1) | 0b00000011;
    bit_field |= (bit_field_lengths.payload_length - 1) | 0b00001100;
    bit_field |= (bit_field_lengths.timestamp_length - 1) | 0b0111000;

    bit_field
}

fn size_to_num_bytes(length: u64) -> u8 {
    if length <= u8::MAX as u64 {
        1
    } else if length <= u16::MAX as u64 {
        2
    } else if length <= u16::MAX as u64 * u8::MAX as u64 {
        3
    } else if length <= u32::MAX as u64 {
        4
    } else if length <= u32::MAX as u64 * u8::MAX as u64 {
        5
    } else if length <= u32::MAX as u64 * u16::MAX as u64 {
        6
    } else if length <= u32::MAX as u64 * u16::MAX as u64 * u8::MAX as u64 {
        7
    } else {
        8
    }
}
