pub trait Hasher {
    fn hash(x: i32, z: i32, mask: i32) -> i32;
}

// /**
//  * returns the hashed key given the original key
//  */
// private static int getHashedKey(long originalKey)
// {
// return hash((int)(originalKey ^ originalKey >>> 32));
// }
//
// /**
//  * the hash function
//  */
// private static int hash(int smallKey)
// {
// smallKey = smallKey ^ smallKey >>> 20 ^ smallKey >>> 12;
// return smallKey ^ smallKey >>> 7 ^ smallKey >>> 4;
// }

pub struct HashV1_8;
impl Hasher for HashV1_8 {
    fn hash(x: i32, z: i32, mask: i32) -> i32 {
        let l = ChunkPos::coords_to_i64(x, z);

        let small_key = (l ^ l >> 32) as u32;
        let small_key = small_key ^ (small_key >> 20) ^ (small_key >> 12);
        let hash = small_key ^ (small_key >> 7) ^ (small_key >> 4);

        (hash as i32) & mask
    }
}

pub struct HashV1_12;
impl Hasher for HashV1_12 {
    fn hash(x: i32, z: i32, mask: i32) -> i32 {
        let l = ChunkPos::coords_to_i64(x, z);

        // HashCommon.mix()
        let h = l.wrapping_mul(-7046029254386353131) as u64;
        let h = h ^ (h >> 32);
        let h = (h ^ (h >> 16)) as i64;

        (h as i32) & mask
    }
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

    pub fn hash<H: Hasher>(&self, mask: i32) -> i32 {
        H::hash(self.x, self.z, mask)
    }
}

impl Into<i64> for ChunkPos {
    fn into(self) -> i64 {
        self.to_i64()
    }
}