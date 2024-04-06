#![feature(trait_upcasting)]
use std::io::Error;
use mc_utils::positions::{ChunkPos};
use mc_utils::world::{Dimension, World};

pub mod chunk_viewer;

fn main() -> Result<(), Error> {
    let world = World::new("C:\\Ryan\\Personal\\minecraft\\carpetmod112\\server\\proto-test");

    // world.delete_chunk(ChunkPos::new(1022, 1007), Dimension::Nether)?;
    // world.delete_chunk(ChunkPos::new(1022, 1008), Dimension::Nether)?;
    // world.delete_chunk(ChunkPos::new(1061, 1010), Dimension::Nether)?;
    // world.delete_chunk(ChunkPos::new(1061, 1011), Dimension::Nether)?;
    world.delete_chunk(ChunkPos::new(-301, 621), Dimension::Nether)?;
    world.delete_chunk(ChunkPos::new(-301, 622), Dimension::Nether)?;
    
    Ok(())
}
