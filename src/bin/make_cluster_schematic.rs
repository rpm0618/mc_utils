use std::cmp::Ordering;
use std::collections::{HashSet, HashMap};
use std::io::Error;
use quartz_nbt::{NbtCompound, NbtList};
use mc_utils::chunk::Chunk;
use mc_utils::cluster_finder12::HashClusterSet;
use mc_utils::flood_fill::{flood_fill, spider};
use mc_utils::litematica::{LitematicaBuilder, LitematicaRegionBuilder};
use mc_utils::positions::{BlockPos, ChunkPos, Direction, HashV1_12};
use mc_utils::world::{Dimension, World};

fn does_chunk_have_fire(chunk: &Chunk) -> bool {
    let level_data: &NbtCompound = chunk.data.get("Level").unwrap();
    let sections: &NbtList = level_data.get("Sections").unwrap();

    for i in 0..sections.len() {
        let section: &NbtCompound = sections.get(i).unwrap();
        let blocks: &[u8] = section.get("Blocks").unwrap();
        for index in 0..4096 {
            let block_id = blocks[index];
            if block_id == 51 { // fire
                return true;
            }
        }
    }
    false
}
fn get_fireless_chunks(world: &mut World, from: ChunkPos, to: ChunkPos) -> Result<HashSet<ChunkPos>, Error> {
    let mut fireless_chunks = HashSet::new();

    for x in from.x..=to.x {
        for z in from.z..=to.z {
            let pos = ChunkPos::new(x, z);
            if let Some(chunk) = world.get_chunk(pos, Dimension::Nether)? {
                if !does_chunk_have_fire(chunk) {
                    fireless_chunks.insert(pos);
                }
            }
        }
    }

    Ok(fireless_chunks)
}

fn get_highest_block_at(chunk: &Chunk, x: i32, z: i32) -> i32 {
    let mut y = 255;
    while chunk.block_at(BlockPos::new(x, y, z)) == 0 && y > 0 {
        y -= 1;
    }
    y
}

fn main() -> Result<(), Error> {
    let mut world = World::new("C:\\Ryan\\Personal\\minecraft\\carpetmod112\\server\\world");

    let fireless_chunks = get_fireless_chunks(&mut world, ChunkPos::new(-158, -177), ChunkPos::new(-38, -53))?;
    let flood_area = flood_fill((-98, -102).into(), &fireless_chunks, 92);

    let mut cluster_set = HashClusterSet::new(4095);
    for (pos, _) in &flood_area.0 {
        cluster_set.add_chunk(*pos);
    }

    let mut index = 0;
    let mut largest_length = 0;

    for i in 0..cluster_set.intervals.len() {
        let interval = &cluster_set.intervals[i];
        if interval.chunks.len() > largest_length {
            index = i;
            largest_length = interval.chunks.len();
        }
    }

    let mut cluster = cluster_set.intervals[index].chunks.clone();
    let mut left_over: Vec<ChunkPos> = cluster.drain(1200..).collect();
    println!("Biggest Cluster: Start: {}, Length: {}", cluster[0].hash::<HashV1_12>(4095), cluster.len());
    left_over.sort_by(|a, b| {
        if a.x < b.x {
            Ordering::Less
        } else if a.x > b.x {
            Ordering::Greater
        } else {
            if a.z < b.z {
                Ordering::Less
            } else if a.z > b.z {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
    });
    let left_over_json =  format!("[{}]", left_over.iter().map(|pos| format!("[{},{}]", pos.x, pos.z)).collect::<Vec<_>>().join(","));
    println!("Left Over: {}", left_over_json);

    let mut region = LitematicaRegionBuilder::new();
    for chunk in &left_over {
        let hopper_x = chunk.x << 4 | 8;
        let hopper_y = 128;
        let hopper_z = chunk.z << 4 | 8;
        region.set_block((hopper_x, hopper_y, hopper_z).into(), "sand".into(), HashMap::new());
    }

    let mut litematic = LitematicaBuilder::new();
    litematic.add_region("Anti-Cluster", region);
    litematic.save("C:\\Ryan\\Personal\\minecraft\\MultiMC\\instances\\1.12.22\\.minecraft\\schematics\\anti-cluster.litematic", "Anti-Cluster");
    println!("Origin {:?}", litematic.get_origin());

    // let mut region = LitematicaRegionBuilder::new();
    // for target in &cluster {
    //     let hopper_x = target.x << 4 | 8;
    //     let hopper_y = i32::max(get_highest_block_at(world.get_chunk(*target, Dimension::Nether)?.unwrap(), 8, 8) + 1, 128);
    //     let hopper_z = target.z << 4 | 8;
    //     region.set_block(hopper_x, hopper_y, hopper_z, "hopper".into(), HashMap::new());
    //     // println!("Target {:?}", target);
    // }
    //
    // spider(&cluster, &flood_area, |chunk, direction| {
    //     let block_offset = match direction {
    //         Direction::North => {(8, 15)}
    //         Direction::South => {(7, 0)}
    //         Direction::East => {(0, 8)}
    //         Direction::West => {(15, 7)}
    //     };
    //
    //     let block_x = chunk.x << 4 | block_offset.0;
    //     let block_y = i32::max(get_highest_block_at(world.get_chunk(chunk, Dimension::Nether).unwrap().unwrap(), block_offset.0, block_offset.1) + 1, 128);
    //     let block_z = chunk.z << 4 | block_offset.1;
    //     region.set_block(block_x, block_y, block_z, "chest".into(), HashMap::new());
    //     // println!("Spider {:?}", chunk);
    // });
    //
    // let mut litematic = LitematicaBuilder::new();
    // litematic.add_region("Cluster", region);
    // litematic.save("C:\\Ryan\\Personal\\minecraft\\MultiMC\\instances\\1.12.22\\.minecraft\\schematics\\cluster.litematic", "Cluster");
    //
    // println!("Origin {:?}", litematic.get_origin());

    Ok(())
}