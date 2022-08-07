use std::collections::BTreeMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Savefile {
    pub directories: BTreeMap<String, BTreeMap<String, SavefileEntryValue>>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "content"))]
pub enum SavefileEntryValue {
    AssocList(Vec<ListEntry>),
    FlatList(Vec<SavefileEntryValue>),
    Null,
    Number(f32),
    String(String),
    Typepath(String),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ListEntry {
    WithKey {
        key: SavefileEntryValue,
        value: SavefileEntryValue,
    },

    Value(SavefileEntryValue),
}
