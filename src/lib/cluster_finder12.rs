use std::cmp::{max, min};
use crate::chunk_pos::{ChunkPos, HashV1_12};

#[derive(Debug, Clone)]
pub struct HashClusterInterval {
    pub min_hash: i32,
    pub chunks: Vec<ChunkPos>
}

impl HashClusterInterval {
    fn new(min_hash: i32, initial_chunk: ChunkPos) -> HashClusterInterval {
        HashClusterInterval { min_hash, chunks: vec![initial_chunk] }
    }

    fn update_interval(&mut self, chunk: ChunkPos, hash: i32) {
        self.min_hash = min(self.min_hash, hash);
        self.chunks.push(chunk);
    }

    fn merge_intervals(interval1: &HashClusterInterval, interval2: &HashClusterInterval) -> HashClusterInterval {
        let mut chunks = interval1.chunks.clone();
        chunks.extend(interval2.chunks.iter());
        HashClusterInterval {
            min_hash: min(interval1.min_hash, interval2.min_hash),
            chunks
        }
    }
}

#[derive(Debug, Clone)]
pub struct HashClusterSet {
    pub intervals: Vec<HashClusterInterval>,
    mask: i32
}
impl HashClusterSet {
    pub fn new(mask: i32) -> HashClusterSet {
        HashClusterSet { mask, intervals: vec![] }
    }

    fn consolidate_intervals(&mut self, index: usize) {
        let cur_hash = self.intervals[index].min_hash;

        if index + 1 < self.intervals.len() {
            let prev_hash = self.intervals[index + 1].min_hash;
            let prev_len = self.intervals[index + 1].chunks.len() as i32;
            if  prev_hash + prev_len >= cur_hash {
                self.intervals[index + 1] = HashClusterInterval::merge_intervals(&self.intervals[index], &self.intervals[index + 1]);
                self.intervals.remove(index);
            }
        }

        // Re-fetch info, it might have changed
        let cur_hash = self.intervals[index].min_hash;
        let cur_len = self.intervals[index].chunks.len() as i32;

        if index > 0 {
            let next_hash = self.intervals[index - 1].min_hash;
            if  next_hash <= cur_hash + cur_len {
                self.intervals[index] = HashClusterInterval::merge_intervals(&self.intervals[index], &self.intervals[index - 1]);
                self.intervals.remove(index - 1);
            }
        }
    }

    pub fn add_chunk(&mut self, chunk: ChunkPos) {
        let chunk_hash = chunk.hash::<HashV1_12>(self.mask);

        // TODO: Binary search this shit
        for i in 0..self.intervals.len() {
            let cur_interval = &mut self.intervals[i];

            // If this interval contains the chunk, update the interval with it and consolidate
            if chunk_hash >= cur_interval.min_hash && chunk_hash < (cur_interval.min_hash + (cur_interval.chunks.len() as i32)) {
                cur_interval.update_interval(chunk, chunk_hash);
                self.consolidate_intervals(i);
                return;
            }

            // Since we know the interval list is in order an non-overlapping, when we've fully
            // "passed" the chunk we know no other interval can contain this chunk, add it as a new
            // interval and consolidate
            if chunk_hash > cur_interval.min_hash && chunk_hash >= (cur_interval.min_hash + (cur_interval.chunks.len() as i32)) {
                self.intervals.insert(i, HashClusterInterval::new(chunk_hash, chunk));
                self.consolidate_intervals(i);
                return;
            }
        }

        // If we've gotten to this point, the chunk has a larger hash than any of the current
        // intervals, add a new interval to the end
        self.intervals.push(HashClusterInterval::new(chunk_hash, chunk));
        self.consolidate_intervals(self.intervals.len() - 1);
    }

    pub fn add_area(&mut self, corner1: ChunkPos, corner2: ChunkPos) {
        let lower_x = min(corner1.x, corner2.x);
        let lower_z = min(corner1.z, corner2.z);
        let upper_x = max(corner1.x, corner2.x);
        let upper_z = max(corner1.z, corner2.z);

        for x in lower_x..=upper_x {
            for z in lower_z..=upper_z {
                // Skip chunks on the world diagonal, they can unload
                if x == z || x == -z {
                    continue;
                }

                self.add_chunk(ChunkPos::new(x, z));
            }
        }
    }

    pub fn add_view_distance(&mut self, view_target: ChunkPos, view_distance: i32) {
        let nw_corner = ChunkPos::new(view_target.x - view_distance, view_target.z - view_distance);
        let se_corner = ChunkPos::new(view_target.x + view_distance, view_target.z + view_distance);

        self.add_area(nw_corner, se_corner);
    }
}