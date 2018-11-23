use std::{error, fmt, result};

pub type Result<T> = result::Result<T, TexProError>;

#[derive(Debug)]
pub enum TexProError {
    Generic,
    Image(image::ImageError),
}

impl fmt::Display for TexProError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TexProError::Generic => f.write_str("Generic"),
            TexProError::Image(_) => f.write_str("Image"),
        }
    }
}

impl error::Error for TexProError {
    fn description(&self) -> &str {
        match *self {
            TexProError::Generic => "Unspecified error",
            TexProError::Image(ref e) => e.description(),
        }
    }
}

impl From<image::ImageError> for TexProError {
    fn from(cause: image::ImageError) -> TexProError {
        TexProError::Image(cause)
    }
}
