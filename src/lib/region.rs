use std::collections::HashMap;
use std::io::{Error, Read, Seek, SeekFrom};
use std::fmt::{Display, Formatter};
use crate::positions::ChunkPos;
use crate::chunk::Chunk;

#[derive(Debug, Copy, Clone)]
struct RegionLocation([u8; 4]);

impl RegionLocation {
    pub fn offset(&self) -> u32 {
        u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]])
    }

    pub fn sector_count(&self) -> u8 {
        self.0[3]
    }

    pub fn is_present(&self) -> bool {
        self.0[0] | self.0[1] | self.0[2] | self.0[3] != 0
    }
}

impl Display for RegionLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Offset: {}, Sectors: {})", self.offset(), self.sector_count())
    }
}

#[derive(Debug, Clone)]
pub struct RegionHeader {
    locations: [RegionLocation; 1024],
    timestamps: [u32; 1024],
}

impl RegionHeader {
    pub fn parse<R: Read>(reader: &mut R) -> Result<RegionHeader, Error> {
        let mut header = RegionHeader {
            locations: [RegionLocation([0; 4]); 1024],
            timestamps: [0; 1024]
        };
        for i in 0..1024 {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            header.locations[i] = RegionLocation(buf);
        }
        for i in 0..1024 {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            header.timestamps[i] = u32::from_be_bytes(buf);
        }
        Ok(header)
    }
}

pub struct Region {
    pub header: RegionHeader,
    chunks: HashMap<usize, Chunk>
}

impl Region {
    pub fn parse<R: Read + Seek>(reader: &mut R) -> Result<Region, Error> {
        let header = RegionHeader::parse(reader)?;
        let mut chunks: HashMap<usize, Chunk> = HashMap::new();

        for x in 0..32 {
            for z in 0..32 {
                let index = Region::get_chunk_index(ChunkPos::new(x, z));
                let location = header.locations[index];
                if location.is_present() {
                    reader.seek(SeekFrom::Start((location.offset() * 4096) as u64))?;
                    let chunk = Chunk::parse(reader)?;
                    chunks.insert(index, chunk);
                }
            }
        }

        Ok(Region { header, chunks })
    }

    pub fn get_chunk_index(chunk: ChunkPos) -> usize {
        ((chunk.x & 31) as usize) + (((chunk.z & 31) as usize) * 32)
    }

    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        let index = Region::get_chunk_index(pos);
        self.chunks.get(&index)
    }
}
