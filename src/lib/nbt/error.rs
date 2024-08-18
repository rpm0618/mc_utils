use std::io;
use java_string::Utf8Error;
use num_enum::TryFromPrimitiveError;
use thiserror::Error;
use crate::nbt::TagId;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid NBT Tag ID {0:}")]
    InvalidTagId(#[from] TryFromPrimitiveError<TagId>),
    #[error("Invalid Nbt Root Tag {0:?}")]
    InvalidNbtRoot(TagId),
    #[error("Unexpected End Tag")]
    InvalidNbtEndTag,
    #[error("Invalid Modified Utf8 String")]
    InvalidModifiedUtf8(#[from] Utf8Error),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("Deserialization Error {0}")]
    Custom(String)
}

pub type Result<T> = std::result::Result<T, Error>;