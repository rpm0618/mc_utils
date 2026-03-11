use crate::positions::ChunkPos;

fn spread(hash: i32) -> i32 {
    (hash ^ hash >> 16) & i32::MAX
}

// Compute the coarse location for a given chunk in the backing array of java's ConcurrentHashMap
pub fn bin_for(pos: ChunkPos, mask: i32) -> i32 {
    let hash = spread(pos.hash_code());
    hash & mask
}