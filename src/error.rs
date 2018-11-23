use std::{error, fmt, result};

pub type Result<T> = result::Result<T, TexProError>;

#[derive(Debug)]
pub enum TexProError {
    Image(image::ImageError),
    InconsistentVectorLengths,
    InvalidBufferCount,
    InvalidNodeId,
    SlotOccupied,
    Io(std::io::Error),
}

impl fmt::Display for TexProError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TexProError::Image(_) => f.write_str("Image"),
            TexProError::InconsistentVectorLengths => f.write_str("InconsistentVectorLengths"),
            TexProError::InvalidBufferCount => f.write_str("InvalidBufferCount"),
            TexProError::InvalidNodeId => f.write_str("InvalidNodeId"),
            TexProError::SlotOccupied => f.write_str("SlotOccupied"),
            TexProError::Io(_) => f.write_str("Io"),
        }
    }
}

impl error::Error for TexProError {
    fn description(&self) -> &str {
        match *self {
            TexProError::Image(ref e) => e.description(),
            TexProError::InconsistentVectorLengths => "Lengths of vectors are not consistent",
            TexProError::InvalidBufferCount => "Invalid number of channels",
            TexProError::InvalidNodeId => "Invalid NodeId",
            TexProError::SlotOccupied => "Invalid Slot",
            TexProError::Io(ref e) => e.description(),
        }
    }
}

impl From<image::ImageError> for TexProError {
    fn from(cause: image::ImageError) -> TexProError {
        TexProError::Image(cause)
    }
}

impl From<std::io::Error> for TexProError {
    fn from(cause: std::io::Error) -> TexProError {
        TexProError::Io(cause)
    }
}
