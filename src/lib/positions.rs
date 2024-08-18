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

    pub fn offset(&self, offset: ChunkPos) -> ChunkPos {
        ChunkPos {
            x: self.x + offset.x,
            z: self.z + offset.z
        }
    }
    
    pub fn to_i64(&self) -> i64 {
        ((self.x as i64) & 4294967295) | (((self.z as i64) & 4294967295) << 32)
    }

    pub fn hash(&self, mask: i32) -> i32 {
        let l = self.to_i64();

        // HashCommon.mix()
        let h = l.wrapping_mul(-7046029254386353131) as u64;
        let h = h ^ (h >> 32);
        let h = (h ^ (h >> 16)) as i64;

        (h as i32) & mask
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
    
    pub fn offset(&self, offset: BlockPos) -> BlockPos {
        BlockPos {
            x: self.x + offset.x,
            y: self.y + offset.y,
            z: self.z + offset.z
        }
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