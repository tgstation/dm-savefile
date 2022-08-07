use crate::{decoder::XorDecoder, savefile::*};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    io::Read,
};

const DATA_XOR_KEY: u8 = 0x3A;
const KEY_XOR_KEY: u8 = 0x53;

fn read_bytes<R: Read>(mut reader: R, size: usize) -> std::io::Result<Vec<u8>> {
    let mut buffer = vec![0; size];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

struct Directory {
    name: Option<String>,
    entries: BTreeMap<String, SavefileEntryValue>,
}

struct EntryData {
    index: u32,
    directory_index: u32,
    key_length: u8,

    key: Vec<u8>,
    data: Vec<u8>,
}

fn read_entry_data<R: Read>(mut data: R) -> crate::Result<Option<EntryData>> {
    let entry_header = match read_bytes(&mut data, 5) {
        Ok(entry_header) => entry_header,
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };

    let entry_length = u32::from_le_bytes(entry_header[..4].try_into().unwrap());
    let entry_data = read_bytes(&mut data, entry_length as usize)?;

    // This is just a guess
    if entry_header[4] != 1 {
        return Ok(Some(EntryData {
            index: 0,
            directory_index: 0,
            key_length: 0,
            key: vec![],
            data: vec![],
        }));
    }

    let key_length = entry_data[8];

    // TODO: remove these unwrap()s
    Ok(Some(EntryData {
        index: u32::from_le_bytes(entry_data[..4].try_into().unwrap()),
        directory_index: u32::from_le_bytes(entry_data[4..8].try_into().unwrap()),
        key_length,

        key: entry_data[9..(9 + (key_length as usize))].to_vec(),
        data: entry_data[(9 + (key_length as usize))..].to_vec(),
    }))
}

#[derive(Debug)]
enum ParsedEntry {
    Finished,

    NoKey,

    NewDirectory {
        directory: String,
        index: u32,
    },

    Parsed {
        key: String,
        value: SavefileEntryValue,
        directory: u32,
    },
}

fn read_entry_value(
    reading: Cow<'_, str>,
    value_decoder: &mut XorDecoder,
) -> crate::Result<SavefileEntryValue> {
    Ok(match value_decoder.next() {
        // Null
        Some(0x00) => SavefileEntryValue::Null,

        // String
        Some(0x01) => {
            let length = value_decoder.read_u16().expect("NYI: no length") as usize;
            let value = value_decoder.take(length).collect::<Vec<_>>();

            SavefileEntryValue::String(String::from_utf8_lossy(&value).to_string())
        }

        // Typepath
        Some(0x03) => {
            let length = value_decoder.read_u16().expect("NYI: no length") as usize;
            let value = value_decoder.take(length).collect::<Vec<_>>();

            SavefileEntryValue::Typepath(String::from_utf8_lossy(&value).to_string())
        }

        // Number
        Some(0x04) => SavefileEntryValue::Number(f32::from_le_bytes(
            value_decoder.take(4).collect::<Vec<_>>()[..]
                .try_into()
                .expect("NYI: Not enough bytes"),
        )),

        // List
        Some(0x0D) => {
            let length = value_decoder.read_u32().expect("NYI: no list length") as usize;

            let is_assoc = match value_decoder.next() {
                Some(0x00) => false,
                Some(0x01) => true,
                Some(other) => unreachable!("is_assoc = {other:x}"),
                None => {
                    return Err(crate::Error::ListEntryMissingIsAssocBit {
                        key: reading.to_string(),
                        length,
                    })
                }
            };

            if is_assoc {
                // It'll grow, but that's fine
                let mut list = Vec::with_capacity(length);

                // All flat keys are stored at the beginning
                for index in 0..length - 1 {
                    list.push(ListEntry::Value(read_entry_value(
                        Cow::from(format!("{reading} - flat key {index}")),
                        value_decoder,
                    )?));
                }

                // The first key is stored at the end, as if it were a flat key
                let first_key =
                    read_entry_value(Cow::from(format!("{reading} - first key")), value_decoder)?;

                let assoc_length = value_decoder.read_u32().expect("NYI: no assoc_length") as usize;

                // i have no idea what this is
                assert_eq!(value_decoder.next(), Some(0x00));

                list.push(ListEntry::WithKey {
                    key: first_key,
                    value: read_entry_value(
                        Cow::from(format!("{reading} - first value")),
                        value_decoder,
                    )
                    .expect("NYI: no assoc value for first key"),
                });

                for index in 0..assoc_length - 1 {
                    let key = read_entry_value(
                        Cow::from(format!("{reading} - assoc key {index}")),
                        value_decoder,
                    )
                    .expect("NYI: no assoc key");

                    let value = read_entry_value(
                        Cow::from(format!("{reading} - assoc value {index})")),
                        value_decoder,
                    )
                    .expect("NYI: no assoc value");

                    list.push(ListEntry::WithKey { key, value });
                }

                SavefileEntryValue::AssocList(list)
            } else {
                let mut list = Vec::with_capacity(length);

                for index in 0..length {
                    list.push(read_entry_value(
                        Cow::from(format!("{reading} - list entry {index}")),
                        value_decoder,
                    )?);
                }

                SavefileEntryValue::FlatList(list)
            }
        }

        Some(unknown) => {
            return Err(crate::Error::UnknownValueType {
                key: reading.into_owned(),
                value_type: unknown,
            });
        }

        None => {
            todo!("value_type == None");
        }
    })
}

fn read_next_entry<R: Read>(mut data: &mut R) -> crate::Result<ParsedEntry> {
    let entry_data = match read_entry_data(&mut data) {
        Ok(Some(data)) => data,
        Ok(None) => return Ok(ParsedEntry::Finished),
        Err(error) => return Err(error),
    };

    if entry_data.key_length == 0 {
        return Ok(ParsedEntry::NoKey);
    }

    let key_data = XorDecoder::new(&entry_data.key, KEY_XOR_KEY).collect::<Vec<_>>();

    // TODO: Error for invalid utf-8
    let key = String::from_utf8_lossy(&key_data).to_string();

    let data_length = u32::from_le_bytes(
        entry_data.data[..4]
            .try_into()
            .expect("NYI: Not enough bytes"),
    ) as usize;

    let mut value_decoder = XorDecoder::new(&entry_data.data[4..(4 + data_length)], DATA_XOR_KEY);

    let value = match value_decoder.peek() {
        // Empty list
        Some(0x0D) if data_length == 6 => {
            value_decoder.next();
            SavefileEntryValue::FlatList(Vec::new())
        }

        Some(_) => read_entry_value(Cow::from(&key), &mut value_decoder)?,

        // data_length == 0
        None => {
            return Ok(ParsedEntry::NewDirectory {
                directory: key,
                index: entry_data.index,
            })
        }
    };

    Ok(ParsedEntry::Parsed {
        key,
        value,
        directory: entry_data.directory_index,
    })
}

pub fn extract_savefile<R: Read>(mut data: R) -> crate::Result<Savefile> {
    let mut savefile = Savefile::default();

    // read file header
    read_entry_data(&mut data)?;

    let mut directory_indexes: HashMap<u32, Directory> = HashMap::new();
    directory_indexes.insert(
        0,
        Directory {
            name: Some(String::new()),
            entries: BTreeMap::new(),
        },
    );

    loop {
        match read_next_entry(&mut data)? {
            ParsedEntry::Parsed {
                key,
                value,
                directory: directory_index,
            } => {
                let directory =
                    directory_indexes
                        .entry(directory_index)
                        .or_insert_with(|| Directory {
                            name: None,
                            entries: BTreeMap::new(),
                        });

                directory.entries.insert(key.clone(), value);
            }

            ParsedEntry::NoKey => {
                continue;
            }

            ParsedEntry::NewDirectory {
                directory: directory_name,
                index,
            } => {
                directory_indexes
                    .entry(index)
                    .and_modify(|directory| directory.name = Some(directory_name.clone()))
                    .or_insert_with(|| Directory {
                        name: Some(directory_name),
                        entries: BTreeMap::new(),
                    });
            }

            ParsedEntry::Finished => {
                break;
            }
        }
    }

    savefile.directories = directory_indexes
        .into_iter()
        .map(|(index, directory)| {
            Ok((
                match directory.name {
                    Some(name) => name,
                    None => return Err(crate::Error::DirectoryCreatedWithoutName(index)),
                },
                directory.entries,
            ))
        })
        .collect::<crate::Result<_>>()?;

    Ok(savefile)
}
