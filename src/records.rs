use std::collections::HashMap;
use std::time::Duration;

use bytemuck::{AnyBitPattern, try_cast_slice};

use crate::control_records::{
    FinishRecordData, SetMetaDataRecordData, StartRecordData, process_control_record,
};

use crate::shared::{
    WpilogReadErrors, bool_to_byte, next_chunk, next_chunk_slice, no_data_err_if_none,
    pad_to_n_bytes,
};
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EntryMetadata<'a> {
    pub start_record_index: u32,
    name: &'a str,
    data_type: DataType,
    pub metadata: Metadata<'a>,
    pub finish_record_index: Option<u32>,
}

impl EntryMetadata<'_> {
    pub fn new<'a>(
        start_record_index: u32,
        name: &'a str,
        data_type: DataType,
        metadata: Metadata<'a>,
    ) -> EntryMetadata<'a> {
        EntryMetadata {
            start_record_index,
            name,
            data_type,
            metadata,
            finish_record_index: None,
        }
    }
}
#[allow(dead_code)]
#[derive(Debug)]
pub struct Entry<'a> {
    pub meta_data: Vec<EntryMetadata<'a>>,
    pub records: Vec<Record<'a>>,
}
impl Entry<'_> {
    pub fn new<'a>(meta_data: Vec<EntryMetadata<'a>>, records: Vec<Record<'a>>) -> Entry<'a> {
        Entry { meta_data, records }
    }
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Metadata<'a> {
    pub metadata: &'a str,
}

impl Metadata<'_> {
    pub fn to_bytes(&self) -> Result<Vec<u8>, WpilogReadErrors> {
        let mut out = vec![];
        out.extend_from_slice((self.metadata.len() as u32).to_le_bytes().as_slice());
        out.extend_from_slice(self.metadata.as_bytes());
        Ok(out)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Record<'a> {
    pub entry_id: u32,
    pub timestamp: Duration,
    pub data: RecordData<'a>,
}
#[allow(dead_code)]
#[derive(Debug)]
pub enum RecordData<'a> {
    Start(StartRecordData<'a>),
    Finish(FinishRecordData),
    SetMetadata(SetMetaDataRecordData<'a>),
    Raw(&'a [u8]),
    Boolean(bool),
    Integer(i64),
    Float(f32),
    Double(f64),
    String(&'a str),
    BooleanArray(&'a [bool]),
    IntegerArray(Vec<i64>),
    FloatArray(Vec<f32>),
    DoubleArray(Vec<f64>),
    StringArray(Vec<&'a str>),
    Json(&'a str),
    MessagePack(&'a [u8]),
    Struct(&'a [u8]),
    StructArray(&'a [u8]),
    PhotonStruct(&'a [u8]),
    ProtoBuff(&'a [u8]),
    Other(&'a [u8]),
}
impl RecordData<'_> {
    pub fn to_bytes<'a>(&'a self) -> Result<Vec<u8>, WpilogReadErrors> {
        let bytes = match self {
            RecordData::Start(d) => d.to_bytes()?,
            RecordData::Finish(d) => d.to_bytes(),
            RecordData::SetMetadata(d) => d.to_bytes()?,
            RecordData::Raw(d) => d.to_vec(),
            RecordData::Boolean(d) => [bool_to_byte(d)].to_vec(),
            RecordData::Integer(d) => d.to_le_bytes().to_vec(),
            RecordData::Float(d) => d.to_le_bytes().to_vec(),
            RecordData::Double(d) => d.to_le_bytes().to_vec(),
            RecordData::String(d) => [&d.len().to_le_bytes(), d.as_bytes()].concat(),
            RecordData::BooleanArray(d) => {
                convert_array_to_bytes(d, &|boolean: bool| [bool_to_byte(&boolean)]).to_vec()
            }
            RecordData::IntegerArray(d) => convert_array_to_bytes(d, &i64::to_le_bytes).to_vec(),
            RecordData::FloatArray(d) => convert_array_to_bytes(d, &f32::to_le_bytes).to_vec(),
            RecordData::DoubleArray(d) => convert_array_to_bytes(d, &f64::to_le_bytes).to_vec(),
            RecordData::StringArray(d) => convert_string_array_to_bytes(d).to_vec(),
            RecordData::Json(d) => d.as_bytes().to_vec(),
            RecordData::MessagePack(d) => d.to_vec(),
            RecordData::Struct(d) => d.to_vec(),
            RecordData::StructArray(d) => d.to_vec(),
            RecordData::PhotonStruct(d) => d.to_vec(),
            RecordData::ProtoBuff(d) => d.to_vec(),
            RecordData::Other(d) => d.to_vec(),
        };

        Ok(bytes)
    }
}
fn convert_array_to_bytes<'a, T: Clone, const DATA_SIZE: usize>(
    array: &[T],
    from_func: &dyn Fn(T) -> [u8; DATA_SIZE],
) -> Vec<u8> {
    let mut out = vec![];
    for entry in array {
        out.extend_from_slice(from_func(entry.clone()).as_slice());
    }
    out
}

fn convert_string_array_to_bytes<'a>(array: &'a [&str]) -> Vec<u8> {
    let mut out = vec![];

    for str in array {
        out.extend_from_slice((str.len() as u32).to_le_bytes().as_slice());
        out.extend_from_slice(str.as_bytes());
    }

    out
}

const STRUCT_STR: &str = "struct:";
const PROTOBUFF_STR: &str = "proto:";
const PHOTONSTRUCT_STR: &str = "photonstruct:";
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DataType {
    Raw,
    Boolean,
    Integer,
    Float,
    Double,
    String,
    BooleanArray,
    IntegerArray,
    FloatArray,
    DoubleArray,
    StringArray,
    Json,
    MessagePack,
    Struct(String),
    StructArray(String),
    PhotonStruct(String),
    ProtoBuff(String),
    Other(String),
}
impl DataType {
    pub fn from_str(str: &str) -> Result<DataType, WpilogReadErrors> {
        let data_type = match str {
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
            _ => process_structs_and_stuff_type_from_string(str)?,
        };
        Ok(data_type)
    }
    pub fn to_str(&self) -> String {
        match self {
            DataType::Raw => "raw".to_string(),
            DataType::Boolean => "boolean".to_string(),
            DataType::Integer => "int64".to_string(),
            DataType::Float => "float".to_string(),
            DataType::Double => "double".to_string(),
            DataType::String => "string".to_string(),
            DataType::BooleanArray => "boolean[]".to_string(),
            DataType::IntegerArray => "int64[]".to_string(),
            DataType::FloatArray => "float[]".to_string(),
            DataType::DoubleArray => "double[]".to_string(),
            DataType::StringArray => "string[]".to_string(),
            DataType::Json => "json".to_string(),
            DataType::MessagePack => "msgpack".to_string(),
            DataType::Struct(s) => STRUCT_STR.to_owned() + s,
            DataType::StructArray(s) => STRUCT_STR.to_owned() + s + "[]",
            DataType::ProtoBuff(s) => PROTOBUFF_STR.to_owned() + s,
            DataType::PhotonStruct(s) => PHOTONSTRUCT_STR.to_owned() + s,
            DataType::Other(s) => s.to_string(),
        }
    }
}
fn process_structs_and_stuff_type_from_string(str: &str) -> Result<DataType, WpilogReadErrors> {
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
    if str.starts_with(PROTOBUFF_STR) {
        return Ok(DataType::ProtoBuff(
            str.split_at(PROTOBUFF_STR.len()).1.to_string(),
        ));
    }
    if str.starts_with(PHOTONSTRUCT_STR) {
        return Ok(DataType::PhotonStruct(
            str.split_at(PHOTONSTRUCT_STR.len()).1.to_string(),
        ));
    }

    Ok(DataType::Other(str.to_owned()))
}

fn get_current_entry_data<'a>(
    entry_lut: &'a HashMap<u32, Entry<'a>>,
    current_record: u32,
    entry_id: u32,
) -> Result<&'a EntryMetadata<'a>, WpilogReadErrors> {
    let current_id_data = match entry_lut.get(&entry_id) {
        Some(l) => &l.meta_data,
        None => {
            return Err(WpilogReadErrors::UseOfEntryIdWithoutStart {
                record_num: current_record,
                entry_id,
            });
        }
    };

    for data in current_id_data {
        match data.finish_record_index {
            None => return Ok(data),
            Some(i) => {
                if i < current_record {
                    return Ok(data);
                }
            }
        }
    }

    Err(WpilogReadErrors::UseOfEntryIdAfterFinish {
        record_num: current_record,
        entry_id,
    })
}

pub fn read_next_record<'a>(
    file: &mut &'a [u8],
    entry_lut: &mut HashMap<u32, Entry<'a>>,
    current_record: u32,
) -> Result<(), WpilogReadErrors> {
    let header_bit_field = no_data_err_if_none!(next_chunk::<1>(file))[0];

    let entry_id = u32::from_le_bytes(pad_to_n_bytes(no_data_err_if_none!(next_chunk_slice(
        file,
        ((header_bit_field & 0b000011) + 1) as usize,
    ))));

    let raw_payload_size = pad_to_n_bytes(no_data_err_if_none!(next_chunk_slice(
        file,
        (((header_bit_field & 0b00001100) >> 2) + 1) as usize,
    )));
    let payload_size = u32::from_le_bytes(raw_payload_size);

    let time_stamp_microseconds = u64::from_le_bytes(pad_to_n_bytes(no_data_err_if_none!(
        next_chunk_slice(file, (((header_bit_field & 0b01110000) >> 4) + 1) as usize,)
    )));
    let time_stamp = Duration::from_micros(time_stamp_microseconds);

    let data = if entry_id == 0 {
        process_control_record(file, current_record, entry_lut)?
    } else {
        let raw_data = no_data_err_if_none!(next_chunk_slice(file, payload_size as usize));
        process_data_from_standard_record(
            &get_current_entry_data(entry_lut, current_record, entry_id)?.data_type,
            raw_data,
            current_record,
            entry_id,
        )?
    };

    let record = Record {
        entry_id: entry_id as u32,
        timestamp: time_stamp,
        data,
    };

    if entry_id != 0 {
        match entry_lut.get_mut(&entry_id) {
            Some(r) => r.records.push(record),
            None => {
                return Err(WpilogReadErrors::UseOfEntryIdWithoutStart {
                    record_num: current_record,
                    entry_id,
                });
            }
        };
    }

    Ok(())
}

fn process_boolean(
    byte: [u8; 1],
    current_record: u32,
    entry_id: u32,
) -> Result<bool, WpilogReadErrors> {
    match byte[0] {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(WpilogReadErrors::MalformedData {
            record_num: current_record,
            entry_id,
        }),
    }
}

fn process_data_from_standard_record<'a>(
    data_type: &DataType,
    data: &'a [u8],
    current_record: u32,
    entry_id: u32,
) -> Result<RecordData<'a>, WpilogReadErrors> {
    let data_type = match data_type {
        DataType::Raw => RecordData::Raw(data),
        DataType::Boolean => RecordData::Boolean(process_boolean(
            data.try_into().unwrap(),
            current_record,
            entry_id,
        )?),
        DataType::Integer => RecordData::Integer(i64::from_le_bytes(data.try_into().unwrap())),
        DataType::Float => RecordData::Float(f32::from_le_bytes(data.try_into().unwrap())),
        DataType::Double => RecordData::Double(f64::from_le_bytes(data.try_into().unwrap())),
        DataType::String => RecordData::String(match str::from_utf8(data) {
            Ok(s) => s,
            Err(_) => {
                return Err(WpilogReadErrors::MalformedData {
                    record_num: current_record,
                    entry_id,
                });
            }
        }),
        DataType::BooleanArray => {
            RecordData::BooleanArray(process_boolean_array_data(data, current_record, entry_id)?)
        }
        DataType::IntegerArray => RecordData::IntegerArray(process_array_data_no_err(
            data,
            &i64::from_le_bytes,
            current_record,
            entry_id,
        )?),
        DataType::FloatArray => RecordData::FloatArray(process_array_data_no_err(
            data,
            &f32::from_le_bytes,
            current_record,
            entry_id,
        )?),
        DataType::DoubleArray => RecordData::DoubleArray(process_array_data_no_err(
            data,
            &f64::from_le_bytes,
            current_record,
            entry_id,
        )?),
        DataType::StringArray => match process_string_array(data, current_record, entry_id) {
            Ok(sa) => sa,
            Err(_) => {
                return Err(WpilogReadErrors::MalformedData {
                    record_num: current_record,
                    entry_id,
                });
            }
        },
        DataType::Json => RecordData::Json(match str::from_utf8(data) {
            Ok(s) => s,
            Err(_) => {
                return Err(WpilogReadErrors::InvalidRecoard {
                    record_num: current_record,
                    entry_id: current_record,
                });
            }
        }),
        DataType::MessagePack => RecordData::MessagePack(data),
        DataType::Struct(_) => RecordData::Struct(data),
        DataType::StructArray(_) => RecordData::StructArray(data),
        DataType::PhotonStruct(_) => RecordData::PhotonStruct(data),
        DataType::ProtoBuff(_) => RecordData::ProtoBuff(data),
        DataType::Other(_) => RecordData::Other(data),
    };
    Ok(data_type)
}

fn process_boolean_array_data<'a>(
    data: &'a [u8],
    current_record: u32,
    entry_id: u32,
) -> Result<&'a [bool], WpilogReadErrors> {
    for d in data {
        if !(*d == 0 || *d == 1) {
            return Err(WpilogReadErrors::MalformedData {
                record_num: current_record,
                entry_id,
            });
        }
    }

    Ok(unsafe { core::slice::from_raw_parts(data.as_ptr() as *const bool, data.len()) })
}

fn process_array_data_no_err<T, const DATA_SIZE: usize>(
    data: &[u8],
    from_func: &dyn Fn([u8; DATA_SIZE]) -> T,
    current_record: u32,
    entry_id: u32,
) -> Result<Vec<T>, WpilogReadErrors> {
    let mut out = Vec::new();
    let mut entries = data.chunks_exact(DATA_SIZE);

    for e in entries.by_ref() {
        out.push(from_func(e.try_into().unwrap()));
    }
    if !entries.remainder().is_empty() {
        return Err(WpilogReadErrors::MalformedData {
            record_num: current_record,
            entry_id,
        });
    }
    Ok(out)
}

fn process_array_data<T, const DATA_SIZE: usize>(
    data: Vec<u8>,
    from_func: &dyn Fn([u8; DATA_SIZE]) -> Result<T, WpilogReadErrors>,
    current_record: u32,
    entry_id: u32,
) -> Result<Vec<T>, WpilogReadErrors> {
    let mut out = Vec::new();
    let mut entries = data.chunks_exact(DATA_SIZE);

    for e in entries.by_ref() {
        out.push(from_func(e.try_into().unwrap())?);
    }
    if !entries.remainder().is_empty() {
        return Err(WpilogReadErrors::MalformedData {
            record_num: current_record,
            entry_id,
        });
    }
    Ok(out)
}

fn process_string_array<'a>(
    data: &'a [u8],
    current_record: u32,
    entry_id: u32,
) -> Result<RecordData<'a>, WpilogReadErrors> {
    let mut indexer = data;
    let length = u32::from_le_bytes(no_data_err_if_none!(next_chunk(&mut indexer)));
    let mut out = Vec::new();

    for _i in 0..length {
        let string_length = u32::from_le_bytes(no_data_err_if_none!(next_chunk(&mut indexer)));
        out.push(
            match str::from_utf8(no_data_err_if_none!(next_chunk_slice(
                &mut indexer,
                string_length as usize
            ))) {
                Ok(s) => s,
                Err(_) => {
                    return Err(WpilogReadErrors::MalformedData {
                        record_num: current_record,
                        entry_id,
                    });
                }
            },
        );
    }

    Ok(RecordData::StringArray(out))
}

pub fn process_metadata(
    data: &[u8],
    current_record: u32,
    entry_id: u32,
) -> Result<Metadata<'_>, WpilogReadErrors> {
    let metadata = match str::from_utf8(data) {
        Ok(j) => j,
        Err(_) => {
            return Err(WpilogReadErrors::MalformedData {
                record_num: current_record,
                entry_id,
            });
        }
    };
    Ok(Metadata { metadata })
}
