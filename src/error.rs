use std::{fmt, io, result};

pub type Result<T> = result::Result<T, TexProError>;

#[derive(Debug)]
pub enum TexProError {
    Generic, // Should come with an error message
    Image(image::ImageError),
    InvalidBufferCount,
    InvalidNodeId,
    InvalidNodeType,
    InvalidSlotId,
    InvalidSlotType,
    InvalidEdge,
    NoSlotData,
    SlotOccupied,
    SlotNotOccupied,
    UnableToLock,
    NodeProcessing,
    PoisonError,
    TryLockError,
    NodeDirty,
    Io(io::Error),
    InvalidName,
}

impl fmt::Display for TexProError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Generic => f.write_str("Something went wrong"),
            Self::Image(ref e) => e.fmt(f),
            Self::InvalidBufferCount => f.write_str("Invalid number of channels"),
            Self::InvalidNodeId => f.write_str("Invalid `NodeId`"),
            Self::InvalidNodeType => f.write_str("Invalid `NodeType`"),
            Self::InvalidSlotId => f.write_str("Invalid `SlotId`"),
            Self::InvalidSlotType => f.write_str("Invalid `SlotType`"),
            Self::InvalidEdge => f.write_str("Invalid `Edge`"),
            Self::NoSlotData => f.write_str("Could not find a `SlotData`"),
            Self::SlotOccupied => f.write_str("`SlotId` is already in use"),
            Self::SlotNotOccupied => f.write_str("`SlotId` is not in use"),
            Self::UnableToLock => f.write_str("Unable to get a lock"),
            Self::NodeProcessing => f.write_str("Error during node processing"),
            Self::PoisonError => f.write_str("Error with poisoned lock"),
            Self::TryLockError => f.write_str("Error when trying to lock"),
            Self::NodeDirty => f.write_str("The node is not up to date"),
            Self::Io(ref e) => e.fmt(f),
            Self::InvalidName => f.write_str(
                "Invalid name, can only contain lowercase letters, numbers and underscores",
            ),
        }
    }
}

impl From<image::ImageError> for TexProError {
    fn from(cause: image::ImageError) -> TexProError {
        Self::Image(cause)
    }
}

impl From<io::Error> for TexProError {
    fn from(cause: io::Error) -> TexProError {
        Self::Io(cause)
    }
}

impl<T> From<std::sync::PoisonError<T>> for TexProError {
    fn from(_: std::sync::PoisonError<T>) -> TexProError {
        Self::PoisonError
    }
}

impl<T> From<std::sync::TryLockError<T>> for TexProError {
    fn from(_: std::sync::TryLockError<T>) -> TexProError {
        Self::TryLockError
    }
}
