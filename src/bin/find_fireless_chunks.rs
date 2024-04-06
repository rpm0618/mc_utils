use std::fs::File;
use std::io::{BufReader, Error, Write};
use quartz_nbt::{NbtCompound, NbtList};
use mc_utils::positions::{ChunkPos, RegionPos};
use mc_utils::world::{Dimension, World};

use rayon::prelude::*;
use mc_utils::chunk::Chunk;
use mc_utils::region::Region;

fn count_fire_in_chunk(chunk: &Chunk) -> u64 {
    let level_data: &NbtCompound = chunk.data.get("Level").unwrap();
    let sections: &NbtList = level_data.get("Sections").unwrap();

    let mut total_fire = 0;
    for i in 0..sections.len() {
        let section: &NbtCompound = sections.get(i).unwrap();
        // let subchunk: u8 = section.get("Y").unwrap();
        let blocks: &[u8] = section.get("Blocks").unwrap();
        for index in 0..4096 {
            let block_id = blocks[index];
            if block_id == 51 { // fire
                total_fire += 1;
            }
        }
    }
    total_fire
}

fn find_fireless_chunks() -> Result<(), Error> {
    let width = 41;
    let height = 41;
    let fireless_chunks: Vec<_> = (0..=(width * height)).into_par_iter().flat_map(|index| -> Vec<ChunkPos> {
        let x = (index % width) - (width / 2);
        let z = (index / height) - (height / 2);
        let mut fireless_chunks: Vec<ChunkPos> = Vec::new();

        let world = World::new("C:\\Ryan\\Personal\\minecraft\\carpetmod112\\server\\world");
        // let world = World::new("C:\\Ryan\\Personal\\minecraft\\prototech\\f3c16a20-b232-4147-8596-083bcde74833\\world");
        let path = World::get_region_path(&world.world_path, RegionPos::new(x, z), Dimension::Nether);
        if path.exists() {
            println!("Loading region r.{}.{}.mca", x, z);
            let region_file = File::open(path).unwrap();
            let length = region_file.metadata().unwrap().len();
            if length == 0 {
                println!("Size is 0, skipping");
                return Vec::new();
            }
            let mut buf_reader = BufReader::new(region_file);
            let region = Region::parse(&mut buf_reader).unwrap();

            for chunk_x in 0..32 {
                for chunk_z in 0..32 {
                    let chunk = region.get_chunk(ChunkPos::new(chunk_x, chunk_z));
                    if let Some(chunk) = chunk {
                        if count_fire_in_chunk(chunk) == 0 {
                            fireless_chunks.push(ChunkPos::new(x << 5 | chunk_x, z << 5 | chunk_z));
                        }
                    }
                }
            }
        }
        fireless_chunks
    }).collect();

    println!("{} Chunks without fire", fireless_chunks.len());

    let mut output_file = File::create("out.csv")?;
    let mut output_json = File::create("fireless_chunks.json")?;

    output_json.write_all("[".as_bytes())?;
    for (i, chunk) in fireless_chunks.iter().enumerate() {
        let line = format!("{},{}\n", chunk.x, chunk.z);
        output_file.write_all(line.as_bytes())?;

        if i < fireless_chunks.len() - 1 {
            output_json.write_all(format!("[{},{}],", chunk.x, chunk.z).as_bytes())?;
        } else {
            output_json.write_all(format!("[{},{}]", chunk.x, chunk.z).as_bytes())?;
        }
    }
    output_json.write_all("]".as_bytes())?;

    Ok(())
}

fn main() -> Result<(), Error> {
    find_fireless_chunks()
}