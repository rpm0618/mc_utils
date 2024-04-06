pub trait ChunkHasher {
    fn hash(x: i32, z: i32, mask: i32) -> i32;
}

pub struct HashV1_8;

impl ChunkHasher for HashV1_8 {
    fn hash(x: i32, z: i32, mask: i32) -> i32 {
        let l = ChunkPos::coords_to_i64(x, z);

        let small_key = (l ^ l >> 32) as u32;
        let small_key = small_key ^ (small_key >> 20) ^ (small_key >> 12);
        let hash = small_key ^ (small_key >> 7) ^ (small_key >> 4);

        (hash as i32) & mask
    }
}

pub struct HashV1_12;

impl ChunkHasher for HashV1_12 {
    fn hash(x: i32, z: i32, mask: i32) -> i32 {
        let l = ChunkPos::coords_to_i64(x, z);

        // HashCommon.mix()
        let h = l.wrapping_mul(-7046029254386353131) as u64;
        let h = h ^ (h >> 32);
        let h = (h ^ (h >> 16)) as i64;

        (h as i32) & mask
    }
}

#[derive(Copy, Clone)]
pub enum Direction {
    North,
    South,
    East,
    West
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32
}

impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        ChunkPos { x, z }
    }

    pub fn to_i64(&self) -> i64 {
        ChunkPos::coords_to_i64(self.x, self.z)
    }

    pub fn coords_to_i64(x: i32, z: i32) -> i64 {
        ((x as i64) & 4294967295) | (((z as i64) & 4294967295) << 32)
    }

    pub fn hash<H: ChunkHasher>(&self, mask: i32) -> i32 {
        H::hash(self.x, self.z, mask)
    }
}
impl From<(i32, i32)> for ChunkPos {
    fn from((x, z): (i32, i32)) -> Self {
        ChunkPos {x, z}
    }
}
impl Into<i64> for ChunkPos {
    fn into(self) -> i64 {
        self.to_i64()
    }
}
impl From<RegionPos> for ChunkPos {
    fn from(value: RegionPos) -> Self {
        ChunkPos { x: value.x << 5, z: value.z << 5 }
    }
}
impl From<BlockPos> for ChunkPos {
    fn from(value: BlockPos) -> Self {
        ChunkPos { x: value.x >> 4, z: value.z >> 4 }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RegionPos {
    pub x: i32,
    pub z: i32
}
impl RegionPos {
    pub fn new(x: i32, z: i32) -> RegionPos {
        RegionPos { x, z }
    }
}
impl From<(i32, i32)> for RegionPos {
    fn from((x, z): (i32, i32)) -> Self {
        RegionPos {x, z}
    }
}
impl From<ChunkPos> for RegionPos {
    fn from(value: ChunkPos) -> Self {
        RegionPos { x: value.x >> 5, z: value.z >> 5}
    }
}
impl From<BlockPos> for RegionPos {
    fn from(value: BlockPos) -> Self {
        RegionPos { x: value.x >> 9, z: value.z >> 9 }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32
}
impl BlockPos {
    pub fn new(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }
}
impl From<(i32, i32, i32)> for BlockPos {
    fn from((x, y, z): (i32, i32, i32)) -> Self {
        BlockPos {x, y, z}
    }
}
impl From<ChunkPos> for BlockPos {
    fn from(value: ChunkPos) -> Self {
        BlockPos { x: value.x << 4, y: 0, z: value.z << 4}
    }
}
impl From<RegionPos> for BlockPos {
    fn from(value: RegionPos) -> Self {
        BlockPos { x: value.x << 9, y: 0, z: value.z << 9 }
    }
}