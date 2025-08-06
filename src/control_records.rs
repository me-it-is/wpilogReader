pub mod control_records {
use std::collections::HashMap;
use crate::shared::shared::{WpilogReadErrors, next_chunk};
use crate::records::records::{Metadata, RecordData, EntryData, DataType, process_metadata};
pub enum ControlTypes {
    Start,
    Finish,
    SetMetadata,
}
#[allow(dead_code)]
pub struct StartRecordData {
    entry_id_to_be_started: u32,
    entry_name: String,
    entry_type: DataType,
    entry_metadata: Metadata,
}
#[allow(dead_code)]
pub struct FinishRecordData {
    entry_to_be_finished: u32,
}
#[allow(dead_code)]
pub struct SetMetaDataRecordData {
    entry_to_be_edited: u32,
    entry_new_metadata: Metadata,
}
pub fn process_control_record(
    file: &mut (Vec<u8>, usize),
    current_record: u32,
    entry_data_lut: &mut HashMap<u32, Vec<EntryData>>,
) -> Result<RecordData, WpilogReadErrors> {
    let raw_control_type = next_chunk(file, 1)?[0];
    let control_type = match raw_control_type {
        0 => ControlTypes::Start,
        1 => ControlTypes::Finish,
        2 => ControlTypes::SetMetadata,
        _ => return Err(WpilogReadErrors::InvalidRecoard),
    };

    return match control_type {
        ControlTypes::Start => process_start_recoard(file, current_record, entry_data_lut),
        ControlTypes::Finish => process_finish_recoard(file, current_record, entry_data_lut),
        ControlTypes::SetMetadata => process_set_metadata_recoard(file, entry_data_lut),
    };
}

fn process_start_recoard(
    file: &mut (Vec<u8>, usize),
    current_record: u32,
    entry_data_lut: &mut HashMap<u32, Vec<EntryData>>,
) -> Result<RecordData, WpilogReadErrors> {
    let entry_id_to_be_started = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());

    let entry_name_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());
    let entry_name_string_raw = next_chunk(file, entry_name_length as usize)?;
    let entry_name_string = match str::from_utf8(entry_name_string_raw.as_slice()) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidRecoard),
    };

    let entry_type_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());
    let entry_type_string_raw = next_chunk(file, entry_type_length as usize)?;
    let entry_type_string = match str::from_utf8(entry_type_string_raw.as_slice()) {
        Ok(s) => s,
        Err(_) => return Err(WpilogReadErrors::InvalidRecoard),
    };

    let entry_type = match entry_type_string {
        "raw" => DataType::Raw,
        "boolean" => DataType::Boolean,
        "int64" => DataType::Integer,
        "float" => DataType::Float,
        "double" => DataType::Double,
        "string" => DataType::String,
        "boolean[]" => DataType::BooleanArray,
        "int64[]" => DataType::IntegerArray,
        "float[]" => DataType::FloatArray,
        "double[]" => DataType::DoubleArray,
        "string[]" => DataType::StringArray,
        "json" => DataType::Json,
        "msgpack" => DataType::MessagePack,
        str => process_structs_and_stuff_type_from_string(str)?,
    };

    let entry_metadata_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());
    let entry_metadata_raw = next_chunk(file, entry_metadata_length as usize)?;
    let entry_metadata = process_metadata(entry_metadata_raw)?;

    let entry_data = EntryData::new(current_record, entry_name_string.to_string(), entry_type.clone(), entry_metadata.clone());

    match entry_data_lut.get_mut(&entry_id_to_be_started) {
        None => _ = entry_data_lut.insert(entry_id_to_be_started, vec![entry_data]),
        Some(current) => {
            let last_index = current.len() - 1;
            if current[last_index].finish_record_index.is_none() {
                return Err(WpilogReadErrors::EntryIdAlreadyStarted);
            }
            current[last_index] = entry_data
        }
    }

    return Ok(RecordData::Start(StartRecordData {
        entry_id_to_be_started,
        entry_name: entry_name_string.to_string(),
        entry_type: entry_type,
        entry_metadata: entry_metadata,
    }));
}

fn process_structs_and_stuff_type_from_string(str: &str) -> Result<DataType, WpilogReadErrors> {
    const STRUCT_STR: &str = "struct:";

    if str.starts_with(STRUCT_STR) {
        if str.ends_with("[]") {
            let mut string = str.split_at(STRUCT_STR.len()).1.to_string();
            string.truncate(string.len() - 2);
            return Ok(DataType::StructArray(string));
        }
        return Ok(DataType::Struct(
            str.split_at(STRUCT_STR.len()).1.to_string(),
        ));
    }
    if str.starts_with("proto:") {
        return Ok(DataType::ProtoBuff(
            str.split_at("proto:".len()).1.to_string(),
        ));
    }
    if str.starts_with("photonstruct:") {
        return Ok(DataType::PhotonStruct(
            str.split_at("photonstruct:".len()).1.to_string(),
        ));
    }
    return Ok(DataType::Raw);
}

fn process_finish_recoard(
    file: &mut (Vec<u8>, usize),
    current_record: u32,
    entry_data_lut: &mut HashMap<u32, Vec<EntryData>>,
) -> Result<RecordData, WpilogReadErrors> {
    println!("finish rec");
    let entry_id_to_be_finished = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());

    let entry = match match entry_data_lut.get_mut(&entry_id_to_be_finished) {
        None => return Err(WpilogReadErrors::SetMetadataWithoutStart),
        Some(data) => data,
    }
    .last_mut()
    {
        None => return Err(WpilogReadErrors::SetMetadataWithoutStart),
        Some(data) => data,
    };

    if entry.finish_record_index.is_some() {
        return Err(WpilogReadErrors::FinishWithoutStart);
    }
    entry.finish_record_index = Some(current_record);

    return Ok(RecordData::Finish(FinishRecordData {
        entry_to_be_finished: entry_id_to_be_finished,
    }));
}
fn process_set_metadata_recoard(
    file: &mut (Vec<u8>, usize),
    entry_data_lut: &mut HashMap<u32, Vec<EntryData>>,
) -> Result<RecordData, WpilogReadErrors> {
    let entry_id_to_set_metadata = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());

    let entry_metadata_length = u32::from_le_bytes(next_chunk(file, 4)?.try_into().unwrap());
    let entry_metadata_raw = next_chunk(file, entry_metadata_length as usize)?;
    let entry_metadata = process_metadata(entry_metadata_raw)?;

    let entry = match match entry_data_lut.get_mut(&entry_id_to_set_metadata) {
        None => return Err(WpilogReadErrors::SetMetadataWithoutStart),
        Some(data) => data,
    }
    .last_mut()
    {
        None => return Err(WpilogReadErrors::SetMetadataWithoutStart),
        Some(data) => data,
    };

    if entry.finish_record_index.is_some() {
        return Err(WpilogReadErrors::SetMetadataWithoutStart);
    }
    entry.metadata = entry_metadata.clone();

    return Ok(RecordData::SetMetadata(SetMetaDataRecordData {
        entry_to_be_edited: entry_id_to_set_metadata,
        entry_new_metadata: entry_metadata,
    }));
}
}