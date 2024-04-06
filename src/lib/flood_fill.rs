use std::collections::{HashMap, HashSet, VecDeque};
use crate::positions::{ChunkPos, Direction};

pub struct FloodFill(pub HashMap<ChunkPos, u32>);

pub fn flood_fill(start: ChunkPos, safe_chunks: &HashSet<ChunkPos>, max_distance: u32) -> FloodFill {
    let mut flood = HashMap::new();
    let mut work_queue = VecDeque::from([(start, 0u32)]);

    while let Some((pos, dist)) = work_queue.pop_front() {
        if flood.contains_key(&pos) || dist > max_distance {
            continue;
        }
        flood.insert(pos, dist);
        let neighbors = [
            ChunkPos::new(pos.x - 1, pos.z),
            ChunkPos::new(pos.x + 1, pos.z),
            ChunkPos::new(pos.x, pos.z - 1),
            ChunkPos::new(pos.x, pos.z + 1)
        ];
        for chunk in neighbors {
            if safe_chunks.contains(&chunk) && !flood.contains_key(&chunk) {
                work_queue.push_back((chunk, dist + 1));
            }
        }
    }

    FloodFill(flood)
}

pub fn spider<F>(target_chunks: &Vec<ChunkPos>, flood_fill: &FloodFill, mut visitor: F) -> HashSet<ChunkPos>
    where F: FnMut(ChunkPos, Direction)
{
    let mut spider = HashSet::new();

    for target in target_chunks {
        let mut cur_pos = target.clone();
        let cur_distance = flood_fill.0.get(target);

        // Ensure the target is in the flood fill area
        if let Some(cur_distance) = cur_distance {
            let mut cur_distance = *cur_distance;
            while cur_distance > 0 {
                spider.insert(cur_pos);

                // (neighbor chunk, direction it's in)
                let neighbors = [
                    (ChunkPos::new(cur_pos.x - 1, cur_pos.z), Direction::West),
                    (ChunkPos::new(cur_pos.x + 1, cur_pos.z), Direction::East),
                    (ChunkPos::new(cur_pos.x, cur_pos.z - 1), Direction::North),
                    (ChunkPos::new(cur_pos.x, cur_pos.z + 1), Direction::South),
                ];

                // (neighbor chunk, flood fill distance, position in that chunk to place a chest)
                let mut ordered_neighbors = neighbors.map(|(n, p)| { (n, *flood_fill.0.get(&n).unwrap_or(&u32::MAX), p) });
                ordered_neighbors.sort_by_key(|(_, dist, _)| { *dist });

                cur_pos = ordered_neighbors[0].0;
                cur_distance = ordered_neighbors[0].1;
                let direction = ordered_neighbors[0].2;

                visitor(cur_pos, direction);
            }
        }
    }

    spider
}

