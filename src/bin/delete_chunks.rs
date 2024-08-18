#![feature(trait_upcasting)]
use std::io::Error;
use mc_utils::world::{Dimension, World};

pub mod chunk_viewer;

fn main() -> Result<(), Error> {
    // let world = World::new("C:\\Ryan\\Personal\\minecraft\\carpetmod112\\server\\proto-test");
    let world = World::new("C:\\Ryan\\Personal\\minecraft\\mcp\\jars\\EPF test 11 - Copy");

    world.delete_chunk((21, -40).into(), Dimension::Overworld)?;
    world.delete_chunk((21, -41).into(), Dimension::Overworld)?;
    world.delete_chunk((21, -43).into(), Dimension::Overworld)?;
    world.delete_chunk((23, -45).into(), Dimension::Overworld)?;
    
    Ok(())
}
