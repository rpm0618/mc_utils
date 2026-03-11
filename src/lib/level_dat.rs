use std::io::Read;
use flate2::read::GzDecoder;
use crate::error::Result;
use crate::nbt;
use crate::nbt::{visit_nbt, LeafTag, NbtPath, NbtPathElement, NbtVisitor};
use crate::positions::BlockPos;

struct LevelDatData {
    spawn: BlockPos,
    seed: i64
}

struct LevelDatVisitor {
    data: LevelDatData
}
impl LevelDatVisitor {
    fn new() -> LevelDatVisitor {
        LevelDatVisitor {
            data: LevelDatData {
                spawn: BlockPos::new(0, 0, 0),
                seed: 0
            }
        }
    }

    fn visit_data(&mut self, val: LeafTag, path: &NbtPath) -> nbt::Result<()> {
        let Some(NbtPathElement::Element(second_level)) = path.get(2) else {
            return Err(nbt::NbtError::Custom("Unexpected Chunk Structure".to_string()));
        };

        if second_level == "SpawnX" {
            let LeafTag::Int(x) = val else {
                return Err(nbt::NbtError::Custom("Unexpected level.dat Structure, spawn x is not an int".to_string()));
            };
            self.data.spawn.x = x;
        }
        if second_level == "SpawnY" {
            let LeafTag::Int(y) = val else {
                return Err(nbt::NbtError::Custom("Unexpected level.dat Structure, spawn y is not an int".to_string()));
            };
            self.data.spawn.y = y;
        }
        if second_level == "SpawnZ" {
            let LeafTag::Int(z) = val else {
                return Err(nbt::NbtError::Custom("Unexpected level.dat Structure, spawn z is not an int".to_string()));
            };
            self.data.spawn.z = z;
        }
        if second_level == "RandomSeed" {
            let LeafTag::Long(seed) = val else {
                return Err(nbt::NbtError::Custom("Unexpected level.dat Structure, random seed is not a long".to_string()));
            };
            self.data.seed = seed;
        }

        Ok(())
    }
}
impl NbtVisitor for LevelDatVisitor {
    #[inline]
    fn visit_leaf(&mut self, val: LeafTag, path: &NbtPath) -> crate::nbt::Result<()> {
        let Some(NbtPathElement::Element(first_level)) = path.get(1) else {
            return Err(nbt::NbtError::Custom("Unexpected level.dat Structure".to_string()));
        };
        if first_level == "Data" {
            self.visit_data(val, path)?;
        }
        Ok(())
    }
}

pub struct LevelDat {
    pub spawn: BlockPos,
    pub seed: i64
}
impl LevelDat {
    pub fn parse<R: Read>(reader: &mut R) -> Result<LevelDat> {
        let mut visitor = LevelDatVisitor::new();
        let mut reader = GzDecoder::new(reader);
        visit_nbt(&mut reader, &mut visitor).unwrap();

        Ok(LevelDat {
            spawn: visitor.data.spawn,
            seed: visitor.data.seed
        })
    }
}



