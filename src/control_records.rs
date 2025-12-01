use crate::records::{
    DataType, Entry, EntryMetadata, Metadata, RecordData, SubEntry, process_metadata,
};
use crate::shared::{WpilogReadErrors, next_chunk, next_chunk_slice, no_data_err_if_none};

use std::collections::HashMap;
pub enum ControlTypes {
    Start,
    Finish,
    SetMetadata,
}

#[derive(Debug)]
pub struct StartRecordData<'a> {
    entry_id_to_be_started: u32,
    entry_name: &'a str,
    entry_type: DataType,
    entry_metadata: Metadata<'a>,
}

impl StartRecordData<'_> {
    pub fn to_bytes(&self) -> Result<Vec<u8>, WpilogReadErrors> {
        let out = [
            &[0],
            self.entry_id_to_be_started.to_le_bytes().as_slice(),
            (self.entry_name.len() as u32).to_le_bytes().as_slice(),
            self.entry_name.as_bytes(),
            self.entry_metadata.to_bytes()?.as_slice(),
        ]
        .concat();
        Ok(out)
    }
}
#[derive(Debug)]
pub struct FinishRecordData {
    entry_to_be_finished: u32,
}
impl FinishRecordData {
    pub fn to_bytes(&self) -> Vec<u8> {
        [&[1], self.entry_to_be_finished.to_le_bytes().as_slice()].concat()
    }
}

#[derive(Debug)]
pub struct SetMetaDataRecordData<'a> {
    entry_to_be_edited: u32,
    entry_new_metadata: Metadata<'a>,
}
impl SetMetaDataRecordData<'_> {
    pub fn to_bytes(&self) -> Result<Vec<u8>, WpilogReadErrors> {
        let out = [
            &[2],
            self.entry_to_be_edited.to_le_bytes().as_slice(),
            self.entry_new_metadata.to_bytes()?.as_slice(),
        ]
        .concat();
        Ok(out)
    }
}

pub fn process_control_record<'a>(
    file: &mut &'a [u8],
    current_record: u32,
    entry_lut: &mut HashMap<u32, Entry<'a>>,
) -> Result<RecordData<'a>, WpilogReadErrors> {
    let control_type = match next_chunk::<1>(file) {
        Some(value) => value[0],
        None => return Err(WpilogReadErrors::NoDataLeft),
    };

    let control_type = match control_type {
        0 => ControlTypes::Start,
        1 => ControlTypes::Finish,
        2 => ControlTypes::SetMetadata,
        _ => {
            return Err(WpilogReadErrors::InvalidRecoard {
                record_num: current_record,
                entry_id: 0,
            });
        }
    };

    match control_type {
        ControlTypes::Start => process_start_recoard(file, current_record, entry_lut),
        ControlTypes::Finish => process_finish_recoard(file, current_record, entry_lut),
        ControlTypes::SetMetadata => process_set_metadata_recoard(file, current_record, entry_lut),
    }
}

fn process_start_recoard<'a>(
    file: &mut &'a [u8],
    current_record: u32,
    entry_lut: &mut HashMap<u32, Entry<'a>>,
) -> Result<RecordData<'a>, WpilogReadErrors> {
    let entry_id_to_be_started = u32::from_le_bytes(no_data_err_if_none!(next_chunk(file)));

    let entry_name_length = u32::from_le_bytes(no_data_err_if_none!(next_chunk(file)));
    let entry_name = no_data_err_if_none!(next_chunk_slice(file, entry_name_length as usize));
    let entry_name = match str::from_utf8(entry_name) {
        Ok(s) => s,
        Err(_) => {
            return Err(WpilogReadErrors::InvalidRecoard {
                record_num: current_record,
                entry_id: 0,
            });
        }
    };

    let entry_type_length = u32::from_le_bytes(no_data_err_if_none!(next_chunk(file)));
    let entry_type = no_data_err_if_none!(next_chunk_slice(file, entry_type_length as usize));
    let entry_type = match str::from_utf8(entry_type) {
        Ok(s) => s,
        Err(_) => {
            return Err(WpilogReadErrors::InvalidRecoard {
                record_num: current_record,
                entry_id: 0,
            });
        }
    };

    let entry_type = DataType::from_str(entry_type)?;

    let entry_metadata_length = u32::from_le_bytes(no_data_err_if_none!(next_chunk(file)));
    let entry_metadata =
        no_data_err_if_none!(next_chunk_slice(file, entry_metadata_length as usize));
    let entry_metadata = process_metadata(entry_metadata, current_record, entry_id_to_be_started)?;

    match entry_lut.get_mut(&entry_id_to_be_started) {
        None => {
            _ = entry_lut.insert(
                entry_id_to_be_started,
                Entry::new(vec![SubEntry::new(
                    vec![],
                    EntryMetadata::new(entry_name, entry_type.clone(), entry_metadata.clone()),
                )]),
            )
        }
        Some(current) => {
            if !current.sub_entries.last().unwrap().entry_metadata.finished {
                return Err(WpilogReadErrors::EntryIdAlreadyStarted {
                    record_num: current_record,
                    entry_id: entry_id_to_be_started,
                });
            }
            current.sub_entries.push(SubEntry::new(
                vec![],
                EntryMetadata::new(entry_name, entry_type.clone(), entry_metadata.clone()),
            ));
        }
    }

    let record_data = RecordData::Start(StartRecordData {
        entry_id_to_be_started,
        entry_name,
        entry_type,
        entry_metadata,
    });

    Ok(record_data)
}

fn process_finish_recoard<'a>(
    file: &mut &[u8],
    current_record: u32,
    entry_lut: &mut HashMap<u32, Entry>,
) -> Result<RecordData<'a>, WpilogReadErrors> {
    let entry_id_to_be_finished = u32::from_le_bytes(no_data_err_if_none!(next_chunk(file)));

    let entry = match entry_lut.get_mut(&entry_id_to_be_finished) {
        None => {
            return Err(WpilogReadErrors::FinishWithoutStart {
                record_num: current_record,
                entry_id: entry_id_to_be_finished,
            });
        }
        Some(entry) => entry,
    };

    let sub_entry = match entry.sub_entries.last_mut() {
        Some(sub_entry) => sub_entry,
        None => {
            return Err(WpilogReadErrors::FinishWithoutStart {
                record_num: current_record,
                entry_id: entry_id_to_be_finished,
            });
        }
    };

    if sub_entry.entry_metadata.finished {
        return Err(WpilogReadErrors::FinishWithoutStart {
            record_num: current_record,
            entry_id: entry_id_to_be_finished,
        });
    }

    sub_entry.entry_metadata.finished = true;

    Ok(RecordData::Finish(FinishRecordData {
        entry_to_be_finished: entry_id_to_be_finished,
    }))
}

fn process_set_metadata_recoard<'a>(
    file: &mut &'a [u8],
    current_record: u32,
    entry_lut: &mut HashMap<u32, Entry<'a>>,
) -> Result<RecordData<'a>, WpilogReadErrors> {
    let entry_id_to_set_metadata = u32::from_le_bytes(no_data_err_if_none!(next_chunk(file)));

    let entry_metadata_length = u32::from_le_bytes(no_data_err_if_none!(next_chunk(file)));
    let entry_metadata =
        no_data_err_if_none!(next_chunk_slice(file, entry_metadata_length as usize));
    let entry_metadata =
        process_metadata(entry_metadata, current_record, entry_id_to_set_metadata)?;

    let entry = match entry_lut.get_mut(&entry_id_to_set_metadata) {
        None => {
            return Err(WpilogReadErrors::SetMetadataWithoutStart {
                record_num: current_record,
                entry_id: entry_id_to_set_metadata,
            });
        }
        Some(entry) => entry,
    };

    let sub_entry = match entry.sub_entries.last_mut() {
        None => {
            return Err(WpilogReadErrors::SetMetadataWithoutStart {
                record_num: current_record,
                entry_id: entry_id_to_set_metadata,
            });
        }
        Some(sub_entry) => sub_entry,
    };

    sub_entry.entry_metadata.metadata = entry_metadata.clone();

    Ok(RecordData::SetMetadata(SetMetaDataRecordData {
        entry_to_be_edited: entry_id_to_set_metadata,
        entry_new_metadata: entry_metadata,
    }))
}
