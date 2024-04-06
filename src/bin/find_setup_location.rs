use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{Error, Read};
use mc_utils::cluster_finder12::HashClusterSet;
use mc_utils::flood_fill::flood_fill;
use mc_utils::positions::{BlockPos, ChunkPos, HashV1_12};

fn main() -> Result<(), Error> {
    let mut fireless_chunks: HashSet<ChunkPos> = HashSet::new();
    let mut fireless_file = File::open("out.csv")?;
    let mut fireless_str: String = String::new();
    fireless_file.read_to_string(&mut fireless_str)?;
    for line in fireless_str.lines() {
        let coords: Vec<&str> = line.split(",").collect();
        let x = coords[0].parse::<i32>().unwrap();
        let z = coords[1].parse::<i32>().unwrap();
        fireless_chunks.insert(ChunkPos::new(x, z));
    }

    let mut cluster_set = HashClusterSet::new(4095);

    // for x in 75..175 {
    //     for z in -50..50 {
    //         let chunk = ChunkPos::new(x, z);
    //         if fireless_chunks.contains(&chunk) {
    //             cluster_set.add_chunk(chunk);
    //         }
    //     }
    // }

    let flood_area = flood_fill(ChunkPos::new(-98, -102), &fireless_chunks, 92);
    for (pos, _) in &flood_area.0 {
        cluster_set.add_chunk(pos.clone());
    }

    // println!("Score: {}", score_pos(ChunkPos::new(1077, 1027), &cluster_set));

    // let mut best_pos = ChunkPos::new(0, 0);
    // let mut best_score = 0;
    // // for x in 1076..1086 {
    // //     for z in 1025 .. 1035 {
    // for x in -1000..0 {
    //     for z in 0..1000 {
    //         let pos = ChunkPos::new(x, z);
    //         let score = score_pos(pos, &cluster_set);
    //         if score > best_score {
    //             best_score = score;
    //             best_pos = pos;
    //         }
    //     }
    // }

    let best_pos = ChunkPos::new(-283, 639);
    let best_score = score_pos(best_pos, &cluster_set);

    println!("Best Chunk: {:?}", best_pos);
    let block_pos: BlockPos = best_pos.into();
    println!("Position: {:?}", block_pos);
    println!("Best Score: {}", best_score);

    Ok(())
}

fn score_pos(chunk: ChunkPos, cluster_set: &HashClusterSet) -> u64 {
    let mut score = 0;
    for x in -2..=2 {
        for z in -2..=2 {
            if (x == 0 || x == 1) && (z == 0 || z == 1) {
                continue;
            }
            let pos = ChunkPos::new(chunk.x + x, chunk.z + z);
            let cluster = cluster_set.cluster_for(pos);
            if let Some(cluster) = cluster {
                let chunk_score = cluster.clustering_for(pos, cluster_set.get_mask());
                if chunk.x == -283 && chunk.z == 639 {
                    println!("Pos: {:?}, {} {}, {}", pos, cluster.min_hash, cluster.chunks.len(), chunk_score);
                }
                if cluster.min_hash == 1084 {
                    score += chunk_score;
                }
            }
        }
    }
    score
}