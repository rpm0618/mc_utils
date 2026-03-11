use std::cmp::Ordering;
use std::collections::{BinaryHeap};
use indexmap::IndexMap;
use crate::positions::ChunkPos;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct ChunkWithDistance {
    chunk: ChunkPos,
    distance: i32
}
impl ChunkWithDistance {
    fn new(chunk: ChunkPos, cluster_origin: ChunkPos) -> Self {
        ChunkWithDistance {
            chunk,
            distance: (chunk.x - cluster_origin.x).abs() + (chunk.z - cluster_origin.z).abs()
        }
    }
}
impl Ord for ChunkWithDistance {
    fn cmp(&self, other: &Self) -> Ordering {
        other.distance.cmp(&self.distance)
    }
}
impl PartialOrd for ChunkWithDistance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct ChunkWithHash {
    chunk: ChunkWithDistance,
    hash: i32
}
impl ChunkWithHash {
    fn new(chunk: ChunkWithDistance, hash: i32) -> Self {
        ChunkWithHash { chunk, hash }
    }
}
impl PartialOrd for ChunkWithHash {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.chunk.cmp(&other.chunk))
    }
}
impl Ord for ChunkWithHash {
    fn cmp(&self, other: &Self) -> Ordering {
        self.chunk.cmp(&other.chunk)
    }
}

// Find a cluster of chunks suitable for a rehash-based setup. This is different from the
// HashClusterSet in that it better supports clusters that wrap around the end of the hashmap,
// and can filter chunks out so that the cluster will automatically reform after a downsize.

// Chunks are stored in separate buckets based on their hash, sorted based on their distance to the
// cluster origin.
pub struct RehashClusterFinder {
    mask: i32,
    cluster_origin: ChunkPos,
    cluster_target: ChunkPos,
    recluster_on_downsize: bool,
    chunks: IndexMap<i32, BinaryHeap<ChunkWithDistance>>
}
impl RehashClusterFinder {
    pub fn new(mask: i32, cluster_origin: ChunkPos, cluster_target: ChunkPos, recluster_on_downsize: bool) -> Self {
        RehashClusterFinder {
            mask,
            cluster_origin,
            cluster_target,
            recluster_on_downsize,
            chunks: IndexMap::new()
        }
    }

    pub fn add_chunk(&mut self, chunk: ChunkPos) {
        if !self.is_valid_for_cluster(chunk) {
            return;
        }
        let entry = ChunkWithDistance::new(chunk, self.cluster_origin);
        let hash = chunk.hash(self.mask);
        self.chunks.entry(hash).or_insert_with(BinaryHeap::new).push(entry);
    }

    // Only allow chunks that have a hash above the target after an upsize. This ensures that the
    // cluster will reform after a downsize.
    fn is_valid_for_cluster(&self, chunk: ChunkPos) -> bool {
        if !self.recluster_on_downsize {
            return true;
        }
        let next_mask = ((self.mask + 1) * 2) - 1;
        let target_hash = self.cluster_target.hash(next_mask);
        let chunk_hash = chunk.hash(next_mask);
        chunk_hash > target_hash
    }

    pub fn cluster_for(&self, target_chunk: ChunkPos, max_size: usize) -> Vec<ChunkPos> {
        let mut cluster = Vec::new();
        let target_hash = target_chunk.hash(self.mask);

        let mut possible_chunks = IndexMap::new();
        possible_chunks.extend(self.chunks.iter().map(|(k, v)| (*k, v.clone())));

        // Find cluster "support"
        let mut support_hash = target_hash;
        loop {
            if possible_chunks.contains_key(&support_hash) {
                break;
            } else {
                support_hash -= 1;
                if support_hash < 0 {
                    return cluster;
                }
            }
        }

        let mut cur_hash = support_hash;
        while cluster.len() < max_size {
            let mut candidate_chunks = BinaryHeap::new();
            for h in support_hash..=cur_hash {
                if let Some(chunk) = self.peek_chunk(&possible_chunks, h) {
                    candidate_chunks.push(ChunkWithHash::new(*chunk, h));
                }
            }

            let next_chunk = candidate_chunks.pop();
            if let Some(next_chunk) = next_chunk {
                cluster.push(next_chunk.chunk.chunk);
                self.pop_chunk(&mut possible_chunks, next_chunk.hash);
                cur_hash += 1;
            } else {
                // Ran out of chunks to add to the cluster
                break;
            }

            if cluster.len() == max_size {
                break;
            }
        }

        cluster
    }

    fn peek_chunk<'a>(&self, chunks: &'a IndexMap<i32, BinaryHeap<ChunkWithDistance>>, hash: i32) -> Option<&'a ChunkWithDistance> {
        chunks.get(&hash)?.peek()
    }

    fn pop_chunk(&self, chunks: &mut IndexMap<i32, BinaryHeap<ChunkWithDistance>>, hash: i32) -> Option<ChunkWithDistance> {
        let result = chunks.get_mut(&hash)?.pop();
        if chunks.get(&hash)?.is_empty() {
            chunks.shift_remove( &hash);
        }
        result
    }
}