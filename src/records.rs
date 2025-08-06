pub mod records {
    use std::collections::HashMap;
    use std::time::Duration;

    use crate::control_records::control_records::{
        FinishRecordData, SetMetaDataRecordData, StartRecordData, process_control_record,
    };
    use crate::shared::shared::{WpilogReadErrors, next_chunk, pad_to_n_bytes};
    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    pub struct EntryMetadata {
        pub start_record_index: u32,
        name: String,
        data_type: DataType,
        pub metadata: Metadata,
        pub finish_record_index: Option<u32>,
    }

    impl EntryMetadata {
        pub fn new(
            start_record_index: u32,
            name: String,
            data_type: DataType,
            metadata: Metadata,
        ) -> EntryMetadata {
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
    pub struct Entry<'a> {
        pub meta_data: Vec<EntryMetadata>,
        pub records: Vec<&'a Record>,
    }
    impl Entry<'_> {
        pub fn new<'a>(meta_data: Vec<EntryMetadata>, records: Vec<&'a Record>) -> Entry<'a> {
            return Entry { meta_data, records };
        }
    }
    #[allow(dead_code)]
    #[derive(Debug, Clone)]
    pub struct Metadata {
        metadata: Option<serde_json::Value>,
    }

    #[allow(dead_code)]
    pub struct Record {
        entry_id: u32,
        time_stamp: Duration,
        data: RecordData,
    }
    #[allow(dead_code)]
    pub enum RecordData {
        Start(StartRecordData),
        Finish(FinishRecordData),
        SetMetadata(SetMetaDataRecordData),
        Raw(Vec<u8>),
        Boolean(bool),
        Integer(i64),
        Float(f32),
        Double(f64),
        String(String),
        BooleanArray(Vec<bool>),
        IntegerArray(Vec<i64>),
        FloatArray(Vec<f32>),
        DoubleArray(Vec<f64>),
        StringArray(Vec<String>),
        Json(Option<serde_json::Value>),
        MessagePack(Vec<u8>),
        Struct(Vec<u8>),
        StructArray(Vec<u8>),
        PhotonStruct(Vec<u8>),
        ProtoBuff(Vec<u8>),
    }
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
    }
    fn get_current_entry_data<'a>(
        entry_lut: &'a HashMap<u32, Entry>,
        current_record: u32,
        entry_id: u32,
    ) -> Result<&'a EntryMetadata, WpilogReadErrors> {
        let current_id_data = match entry_lut.get(&entry_id) {
            Some(l) => &l.meta_data,
            None => return Err(WpilogReadErrors::UseOfEntryIdWithoutStart),
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

        return Err(WpilogReadErrors::UseOfEntryIdAfterFinish);
    }

    pub fn read_next_record(
        file: &mut (Vec<u8>, usize),
        entry_lut: &mut HashMap<u32, Entry>,
        current_record: u32,
    ) -> Result<Record, WpilogReadErrors> {
        let header_bit_field = next_chunk(file, 1)?[0];

        let entry_id = u32::from_le_bytes(pad_to_n_bytes(next_chunk(
            file,
            ((header_bit_field & 0b000011) + 1) as usize,
        )?));

        let raw_payload_size = pad_to_n_bytes(next_chunk(
            file,
            (((header_bit_field & 0b00001100) >> 2) + 1) as usize,
        )?);
        let payload_size = u32::from_le_bytes(raw_payload_size);

        let timestamp = u64::from_le_bytes(pad_to_n_bytes(next_chunk(
            file,
            (((header_bit_field & 0b01110000) >> 4) + 1) as usize,
        )?));

        let data;

        if entry_id == 0 {
            data = process_control_record(file, current_record, entry_lut)?;
        } else {
            let raw_data = next_chunk(file, payload_size as usize)?;
            data = process_data_from_standard_record(
                &get_current_entry_data(entry_lut, current_record, entry_id)?.data_type,
                raw_data,
            )?;
        }

        return Ok(Record {
            entry_id: entry_id as u32,
            time_stamp: Duration::from_micros(timestamp),
            data: data,
        });
    }

    fn process_boolean(byte: [u8; 1]) -> Result<bool, WpilogReadErrors> {
        return match byte[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => return Err(WpilogReadErrors::MalformedData),
        };
    }

    fn process_data_from_standard_record(
        data_type: &DataType,
        data: Vec<u8>,
    ) -> Result<RecordData, WpilogReadErrors> {
        return Ok(match data_type {
            DataType::Raw => RecordData::Raw(data),
            DataType::Boolean => RecordData::Boolean(process_boolean(data.try_into().unwrap())?),
            DataType::Integer => RecordData::Integer(i64::from_le_bytes(data.try_into().unwrap())),
            DataType::Float => RecordData::Float(f32::from_le_bytes(data.try_into().unwrap())),
            DataType::Double => RecordData::Double(f64::from_le_bytes(data.try_into().unwrap())),
            DataType::String => RecordData::String(match String::from_utf8(data) {
                Ok(s) => s,
                Err(_) => return Err(WpilogReadErrors::MalformedData),
            }),
            DataType::BooleanArray => {
                RecordData::BooleanArray(process_array_data(data, &(process_boolean))?)
            }
            DataType::IntegerArray => {
                RecordData::IntegerArray(process_array_data_no_err(data, &i64::from_le_bytes)?)
            }
            DataType::FloatArray => {
                RecordData::FloatArray(process_array_data_no_err(data, &f32::from_le_bytes)?)
            }
            DataType::DoubleArray => {
                RecordData::DoubleArray(process_array_data_no_err(data, &f64::from_le_bytes)?)
            }
            DataType::StringArray => match process_string_array(data) {
                Ok(sa) => sa,
                Err(_) => return Err(WpilogReadErrors::MalformedData),
            },
            DataType::Json => RecordData::Json(match data.len() {
                0 => None,
                _ => match serde_json::from_slice(&data) {
                    Ok(j) => j,
                    Err(_) => return Err(WpilogReadErrors::MalformedData),
                },
            }),
            DataType::MessagePack => RecordData::MessagePack(data),
            DataType::Struct(_) => RecordData::Struct(data),
            DataType::StructArray(_) => RecordData::StructArray(data),
            DataType::PhotonStruct(_) => RecordData::PhotonStruct(data),
            DataType::ProtoBuff(_) => RecordData::ProtoBuff(data),
        });
    }

    fn process_array_data_no_err<T, const DATA_SIZE: usize>(
        data: Vec<u8>,
        from_func: &dyn Fn([u8; DATA_SIZE]) -> T,
    ) -> Result<Vec<T>, WpilogReadErrors> {
        return process_array_data(data, &|data: [u8; DATA_SIZE]| Ok(from_func(data)));
    }

    fn process_array_data<T, const DATA_SIZE: usize>(
        data: Vec<u8>,
        from_func: &dyn Fn([u8; DATA_SIZE]) -> Result<T, WpilogReadErrors>,
    ) -> Result<Vec<T>, WpilogReadErrors> {
        let mut out = Vec::new();
        let mut entries = data.chunks_exact(DATA_SIZE);

        loop {
            match entries.next() {
                Some(e) => out.push(from_func(e.try_into().unwrap())?),
                None => break,
            }
        }
        if entries.remainder().len() != 0 {
            return Err(WpilogReadErrors::MalformedData);
        }
        return Ok(out);
    }

    fn process_string_array(data: Vec<u8>) -> Result<RecordData, WpilogReadErrors> {
        let mut indexer = (data, 0);
        let length = u32::from_le_bytes(next_chunk(&mut indexer, 4)?.try_into().unwrap());
        let mut out = Vec::new();

        for _i in 0..length {
            let string_length =
                u32::from_le_bytes(next_chunk(&mut indexer, 4)?.try_into().unwrap());
            out.push(
                match String::from_utf8(next_chunk(&mut indexer, string_length as usize)?) {
                    Ok(s) => s,
                    Err(_) => return Err(WpilogReadErrors::MalformedData),
                },
            );
        }

        return Ok(RecordData::StringArray(out));
    }

    pub fn process_metadata(data: Vec<u8>) -> Result<Metadata, WpilogReadErrors> {
        let metadata;
        if data.len() == 0 {
            metadata = None
        } else {
            metadata = Some(match serde_json::from_slice(&data) {
                Ok(j) => j,
                Err(_) => return Err(WpilogReadErrors::MalformedData),
            });
        }
        return Ok(Metadata { metadata });
    }
}
