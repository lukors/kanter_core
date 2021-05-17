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
    SlotOccupied,
    UnableToLock,
    NodeProcessing,
    PoisonError,
    Io(io::Error),
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
            Self::SlotOccupied => f.write_str("`SlotId` is already in use"),
            Self::UnableToLock => f.write_str("Unable to get a lock"),
            Self::NodeProcessing => f.write_str("Error during node processing"),
            Self::PoisonError => f.write_str("Error with poisoned lock"),
            Self::Io(ref e) => e.fmt(f),
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
    fn from(cause: std::sync::PoisonError<T>) -> TexProError {
        Self::PoisonError
    }
}
