use std::cmp::{max, min};
use crate::positions::{ChunkPos, HashV1_12};

#[derive(Debug, Clone)]
pub struct HashClusterInterval {
    pub min_hash: i32,
    // Stored in (increasing) hash order
    pub chunks: Vec<ChunkPos>
}

impl HashClusterInterval {
    fn new(min_hash: i32, initial_chunk: ChunkPos) -> HashClusterInterval {
        HashClusterInterval { min_hash, chunks: vec![initial_chunk] }
    }

    fn contains(&self, hash: i32) -> bool {
        hash >= self.min_hash && hash < (self.min_hash + (self.chunks.len() as i32))
    }

    fn update_interval(&mut self, chunk: ChunkPos, hash: i32, mask: i32) {
        self.min_hash = min(self.min_hash, hash);
        let index = self.chunks.binary_search_by_key(&hash, |c| { c.hash::<HashV1_12>(mask) }).unwrap_or_else(|i| i);
        self.chunks.insert(index, chunk);
    }

    fn merge_intervals(interval1: &HashClusterInterval, interval2: &HashClusterInterval, mask: i32) -> HashClusterInterval {
        let mut chunks = interval1.chunks.clone();
        // Rust sort implementation is optimized for concatenated sorted arrays, we probably
        // can't do better with a custom merge sort
        // https://doc.rust-lang.org/std/vec/struct.Vec.html#method.sort
        chunks.extend(interval2.chunks.iter());
        chunks.sort_by_key(|a| { a.hash::<HashV1_12>(mask)});
        HashClusterInterval {
            min_hash: min(interval1.min_hash, interval2.min_hash),
            chunks
        }
    }

    pub fn clustering_for(&self, chunk: ChunkPos, mask: i32) -> u64 {
        let hash = chunk.hash::<HashV1_12>(mask);
        if !self.contains(hash) {
            return 0
        }
        let index = self.chunks.binary_search_by_key(&hash, |c| c.hash::<HashV1_12>(mask)).unwrap_or_else(|i| i);
        (self.chunks.len() - index) as u64
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

    pub fn get_mask(&self) -> i32 {
        self.mask
    }
    
    fn consolidate_intervals(&mut self, index: usize) {
        let cur_hash = self.intervals[index].min_hash;
        let cur_len = self.intervals[index].chunks.len() as i32;

        if index + 1 < self.intervals.len() {
            let next_hash = self.intervals[index + 1].min_hash;
            if  next_hash <= cur_hash + cur_len {
                self.intervals[index] = HashClusterInterval::merge_intervals(&self.intervals[index], &self.intervals[index + 1], self.mask);
                self.intervals.remove(index + 1);
            }
        }

        // Re-fetch info, it might have changed
        let cur_hash = self.intervals[index].min_hash;

        if index > 0 {
            let prev_hash = self.intervals[index - 1].min_hash;
            let prev_len = self.intervals[index - 1].chunks.len() as i32;
            if  prev_hash + prev_len >= cur_hash {
                self.intervals[index - 1] = HashClusterInterval::merge_intervals(&self.intervals[index], &self.intervals[index - 1], self.mask);
                self.intervals.remove(index);
            }
        }
    }

    pub fn cluster_for(&self, chunk: ChunkPos) -> Option<&HashClusterInterval> {
        let chunk_hash = chunk.hash::<HashV1_12>(self.mask);
        match self.intervals.binary_search_by_key(&chunk_hash, |i| i.min_hash) {
            Ok(index) => Some(&self.intervals[index]),
            Err(index) => {
                if index > 0 && self.intervals[index - 1].contains(chunk_hash) {
                    Some(&self.intervals[index - 1])
                } else {
                    None
                }
            }
        }
    }

    pub fn add_chunk(&mut self, chunk: ChunkPos) {
        // Skip chunks on the world diagonal, they can unload
        if chunk.x == chunk.z || chunk.x == -chunk.z {
            return;
        }

        let chunk_hash = chunk.hash::<HashV1_12>(self.mask);

        match self.intervals.binary_search_by_key(&chunk_hash, |i| i.min_hash) {
            Ok(index) => {
                // If we found an exact match, that means the chunk has the same min_hash as the
                // found interval, and we can just update the interval with that.
                self.intervals[index].update_interval(chunk, chunk_hash, self.mask);
                self.consolidate_intervals(index);
            },
            Err(index) => {
                // If we didn't find an exact match on the min_hash, we first have to check the
                // previous interval to see if we should be a part of that interval. Only if we
                // aren't do we add a new interval
                if index > 0 && self.intervals[index - 1].contains(chunk_hash) {
                    self.intervals[index - 1].update_interval(chunk, chunk_hash, self.mask);
                    self.consolidate_intervals(index - 1);
                } else {
                    self.intervals.insert(index, HashClusterInterval::new(chunk_hash, chunk));
                    self.consolidate_intervals(index);
                }
            }
        }
    }

    pub fn add_area(&mut self, corner1: ChunkPos, corner2: ChunkPos) {
        let lower_x = min(corner1.x, corner2.x);
        let lower_z = min(corner1.z, corner2.z);
        let upper_x = max(corner1.x, corner2.x);
        let upper_z = max(corner1.z, corner2.z);

        for x in lower_x..=upper_x {
            for z in lower_z..=upper_z {
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