use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{Error, Read};
use mc_utils::cluster_finder12::HashClusterSet;
use mc_utils::litematica::{LitematicaBuilder, LitematicaRegionBuilder};
use mc_utils::positions::{ChunkPos, Direction, HashV1_12};
use mc_utils::flood_fill::{flood_fill, spider};


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

    // Wisky perimeter
    // cluster_set.add_area(ChunkPos::new(-138, -84), ChunkPos::new(-114, -62));

    // let flood_area = flood_fill(ChunkPos::new(0, 10), &fireless_chunks, 100);
    // let flood_area = flood_fill(ChunkPos::new(-120, -70), &fireless_chunks, 110);
    // let flood_area = flood_fill(ChunkPos::new(-130, -12), &fireless_chunks);
    // let flood_area = flood_fill(ChunkPos::new(23, -120), &fireless_chunks, 110);
    // let flood_area = flood_fill(ChunkPos::new(73, 81), &fireless_chunks, 120);
    let flood_area = flood_fill(ChunkPos::new(-98, -102), &fireless_chunks, 92);
    for (pos, _) in &flood_area.0 {
        if pos.x == pos.z || pos.x == -pos.z {
            continue;
        }
        cluster_set.add_chunk(pos.clone());
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
    // cluster.insert(0, ChunkPos::new(-8, 5)); // Fill Hole
    // let cluster: Vec<ChunkPos> = cluster.iter().filter(|c| c.hash::<HashV1_12>(4095) > 2621).map(|c| *c).take(400).collect();

    println!("Biggest Cluster: Start: {}, Length: {}", cluster[0].hash::<HashV1_12>(4095), cluster.len());

    let mut region = LitematicaRegionBuilder::new();
    for target in &cluster {
        let hopper_x = target.x << 4 | 8;
        let hopper_y = 128;
        let hopper_z = target.z << 4 | 8;
        region.set_block(hopper_x, hopper_y, hopper_z, "hopper".into(), HashMap::new());
    }
    // let spider = spider(&cluster, &flood_area.0, &mut litematic);
    let spider = spider(&cluster, &flood_area, |chunk, direction| {
       // Position to place a chest to
        let block_offset = match direction {
            Direction::North => {(8, 15)}
            Direction::South => {(7, 0)}
            Direction::East => {(0, 8)}
            Direction::West => {(15, 7)}
        };

        let block_x = chunk.x << 4 | block_offset.0;
        let block_y = 128;
        let block_z = chunk.z << 4 | block_offset.1;
        region.set_block(block_x, block_y, block_z, "chest".into(), HashMap::new())
    });
    let mut litematic = LitematicaBuilder::new();
    litematic.add_region("Cluster Chunks", region);
    litematic.save("C:\\Ryan\\Personal\\minecraft\\MultiMC\\instances\\1.12.22\\.minecraft\\schematics\\cluster.litematic", "Cluster Chunks");

    println!("Spider Loader:");
    println!("Origin {:?}", litematic.get_origin());
    let spider_json = format!("[{}]", spider.iter().map(|pos| format!("[{},{}]", pos.x, pos.z)).collect::<Vec<_>>().join(","));
    println!("{}", spider_json);

    println!("\nCluster:");
    let cluster_json = format!("[{}]", cluster.iter().map(|pos| format!("[{},{}]", pos.x, pos.z)).collect::<Vec<_>>().join(","));
    println!("{}", cluster_json);
    let poke_set = cluster.iter().map(|pos| format!("{},{}", pos.x, pos.z)).collect::<Vec<_>>().join("|");
    println!("{}", poke_set);

    Ok(())
}
