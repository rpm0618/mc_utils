use std::io::{Error};
use quartz_nbt::{NbtCompound, NbtList};
use mc_utils::chunk_pos::ChunkPos;
use mc_utils::cluster_finder12::HashClusterSet;
use mc_utils::world::{Chunk, Dimension, World};

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
                // let x = index & 0xF;
                // let y = (index >> 8) & 0xF;
                // let z = (index >> 4) & 0xF;
                // println!("Found Fire at ({}, {}, {}) in subchunk {}", x, y, z, subchunk);
                total_fire += 1;
            }
        }
    }
    total_fire
}

fn main() -> Result<(), Error> {
    // let mut cluster_set = HashClusterSet::new(4095);
    //
    // // Wisky perimeter
    // cluster_set.add_area(ChunkPos::new(-138, -84), ChunkPos::new(-114, -62));
    //
    // // Lava lake near nether hub
    // cluster_set.add_area(ChunkPos::new(21, -28), ChunkPos::new(37, -12));
    //
    // // Nether hub
    // cluster_set.add_area(ChunkPos::new(-17, -19), ChunkPos::new(16, 16));
    //
    // // Silk touch quarry
    // cluster_set.add_area(ChunkPos::new(81, 7), ChunkPos::new(53, -50));
    //
    // let mut index = 0;
    // let mut largest_length = 0;
    //
    // for i in 0..cluster_set.intervals.len() {
    //     let interval = &cluster_set.intervals[i];
    //     if interval.chunks.len() > largest_length {
    //         index = i;
    //         largest_length = interval.chunks.len();
    //     }
    // }
    //
    // println!("Biggest Cluster: Start: {}, Length: {}", cluster_set.intervals[index].min_hash, largest_length);
    //
    // for chunk in &cluster_set.intervals[index].chunks {
    //     println!("{},{}", chunk.x, chunk.z);
    // }

    // let mut world = World::new("C:\\Ryan\\Personal\\minecraft\\MultiMC\\instances\\1.12.22\\.minecraft\\saves\\superflat test");
    // let chunk = world.get_chunk(ChunkPos::new(62, -6), Dimension::Overworld)?.unwrap();
    // let total_fire = count_fire_in_chunk(&chunk);
    let mut world = World::new("C:\\Ryan\\Personal\\minecraft\\prototech\\backup-08-2023\\world");

    let mut total_fire = 0;
    // Silk Touch Quarry
    for x in 53..=81 {
        for z in -50..=7 {
            let chunk = world.get_chunk(ChunkPos::new(x, z), Dimension::Nether)?.unwrap();
            let chunk_fires = count_fire_in_chunk(chunk);
            total_fire += chunk_fires;
        }
    }

    // Nether Hub
    for x in -17..=16 {
        for z in -19..=16 {
            let chunk = world.get_chunk(ChunkPos::new(x, z), Dimension::Nether)?.unwrap();
            let chunk_fires = count_fire_in_chunk(chunk);
            total_fire += chunk_fires;
        }
    }

    // Nether Hub lava lake {
    for x in 21..=37 {
        for z in -28..=-12 {
            let chunk = world.get_chunk(ChunkPos::new(x, z), Dimension::Nether)?.unwrap();
            let chunk_fires = count_fire_in_chunk(chunk);
            total_fire += chunk_fires;
        }
    }

    // Wisky Perimeter
    for x in -138..=-114 {
        for z in -84..=-62 {
            let chunk = world.get_chunk(ChunkPos::new(x, z), Dimension::Nether)?.unwrap();
            let chunk_fires = count_fire_in_chunk(chunk);
            total_fire += chunk_fires;
        }
    }

    println!("Total Fire: {}", total_fire);

    Ok(())
}
