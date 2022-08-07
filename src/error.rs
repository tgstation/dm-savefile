pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Directory created without a name (for index {0})")]
    DirectoryCreatedWithoutName(u32),

    #[error("List entry had no is_assoc bit when reading {key} (length of {length})")]
    ListEntryMissingIsAssocBit { key: String, length: usize },

    #[error("Error reading file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown value type when reading {key}: {value_type:02x}")]
    UnknownValueType { key: String, value_type: u8 },
}
