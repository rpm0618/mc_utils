use std::io::{Error};
use mc_utils::world::{Dimension, World};


fn main() -> Result<(), Error> {
    let world = World::new("C:\\Ryan\\Personal\\minecraft\\carpetmod112\\server\\world");
    println!("{}", world.get_num_regions(Dimension::Overworld)?);
    Ok(())
}
