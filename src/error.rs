use std::{fmt, io, result};

pub type Result<T> = result::Result<T, TexProError>;

#[derive(Debug)]
pub enum TexProError {
    Image(image::ImageError),
    InvalidBufferCount,
    InvalidNodeId,
    InvalidNodeType,
    InvalidSlotId,
    SlotOccupied,
    NodeProcessing,
    Io(io::Error),
}

impl fmt::Display for TexProError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TexProError::Image(ref e) => e.fmt(f),
            TexProError::InvalidBufferCount => f.write_str("Invalid number of channels"),
            TexProError::InvalidNodeId => f.write_str("Invalid `NodeId`"),
            TexProError::InvalidNodeType => f.write_str("Invalid `NodeType`"),
            TexProError::InvalidSlotId => f.write_str("Invalid `SlotId`"),
            TexProError::SlotOccupied => f.write_str("`SlotId` is already in use"),
            TexProError::NodeProcessing => f.write_str("Error during node processing"),
            TexProError::Io(ref e) => e.fmt(f),
        }
    }
}

impl From<image::ImageError> for TexProError {
    fn from(cause: image::ImageError) -> TexProError {
        TexProError::Image(cause)
    }
}

impl From<io::Error> for TexProError {
    fn from(cause: io::Error) -> TexProError {
        TexProError::Io(cause)
    }
}
