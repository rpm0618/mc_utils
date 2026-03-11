use thiserror::Error;
use crate::nbt;
use crate::positions::{ChunkPos, RegionPos};
use crate::world::Dimension;

#[derive(Error, Debug)]
pub enum McUtilsError {
    #[error("Invalid NBT Compression Type {0:?}")]
    UnknownCompressionType(u8),
    #[error("Invalid chunk format: {0}")]
    InvalidChunkFormat(String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Missing Region File {0:?} {1:?}")]
    MissingRegion(RegionPos, Dimension),
    #[error("Missing Chunk File {0:?} {1:?}")]
    MissingChunk(ChunkPos, Dimension),
    #[error(transparent)]
    NbtError(#[from] nbt::NbtError)
}

pub type Result<T> = std::result::Result<T, McUtilsError>;