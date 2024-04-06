use std::io::{Error, Read};
use quartz_nbt::io::Flavor;
use quartz_nbt::{NbtCompound, NbtList};
use crate::positions::BlockPos;

pub struct Chunk {
    pub length: u32,
    pub compression_type: u8,
    pub data: NbtCompound
}

impl Chunk {
    pub fn parse<R: Read>(reader: &mut R) -> Result<Chunk, Error> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        let length = u32::from_be_bytes(buf);
        let mut compression_type = [0u8];
        reader.read_exact(&mut compression_type)?;

        let flavor = match compression_type[0] {
            1 => Flavor::GzCompressed,
            2 => Flavor::ZlibCompressed,
            3 => Flavor::Uncompressed,
            _ => panic!("Unknown compression type {}", compression_type[0])
        };

        let (data, _): (NbtCompound, _) = quartz_nbt::io::read_nbt(reader, flavor).unwrap();
        Ok(Chunk {
            length,
            compression_type: compression_type[0],
            data
        })
    }

    pub fn block_at(&self, pos: BlockPos) -> u8 {
        let level_data: &NbtCompound = self.data.get("Level").unwrap();
        let sections: &NbtList = level_data.get("Sections").unwrap();
        let subchunk = (pos.y >> 4) as usize;
        if let Ok(section) = sections.get::<&NbtCompound>(subchunk) {
            let blocks: &[u8] = section.get("Blocks").unwrap();
            let x = pos.x & 0xf;
            let y = pos.y & 0xf;
            let z = pos.z & 0xf;
            let index = (x | (y << 8) | (z << 4)) as usize;
            blocks[index]
        } else {
            0
        }
    }
}
