use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::fmt::{Display, Formatter};
use byteorder::{BigEndian, WriteBytesExt};
use crate::positions::ChunkPos;
use crate::chunk::Chunk;
use crate::error::Result;
use crate::raw_chunk::RawChunk;

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
    pub fn parse<R: Read>(reader: &mut R) -> Result<RegionHeader> {
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

    // Builds a bitmap of occupied sectors in the region, and returns the offset of the first block
    // of free sectors that is large enough to fit the given sector count. If none exist, the
    // returned offset will point to the end of the region file.
    pub fn find_free_offset(&self, chunk: ChunkPos, sector_count: usize) -> usize {
        let mut sector_map: Vec<bool> = Vec::new();

        let chunk_index = Region::get_chunk_index(chunk);

        for (i, location) in self.locations.iter().enumerate() {
            if !location.is_present() {
                continue;
            }

            if i == chunk_index {
                continue;
            }

            let offset = location.offset();
            let length = location.sector_count();

            if offset > sector_map.len() as u32 {
                sector_map.resize(offset as usize, false);
                sector_map.resize(offset as usize + length as usize, true);
            } else {
                if offset + length as u32 > sector_map.len() as u32 {
                    sector_map.resize(offset as usize + length as usize, false);
                }
                sector_map[offset as usize..offset as usize + length as usize].iter_mut().for_each(|b| *b = true);
            }
        }

        for start_offset in 2..sector_map.len() {
            let mut free_sectors = 0;
            for sector in sector_map[start_offset..].iter() {
                if !*sector {
                    free_sectors += 1;
                } else {
                    break;
                }
            }
            if free_sectors >= sector_count {
                return start_offset;
            }
        }

        sector_map.len()
    }
}

pub struct Region {
    pub header: RegionHeader,
    chunks: HashMap<usize, Chunk>
}

impl Region {
    pub fn parse<R: Read + Seek>(reader: &mut R) -> Result<Region> {
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

    pub fn get_raw_chunk<R: Read + Seek>(pos: ChunkPos, reader: &mut R) -> Result<Option<RawChunk>> {
        let header = RegionHeader::parse(reader)?;

        let index = Region::get_chunk_index(pos);
        let location = header.locations[index];
        if location.is_present() {
            reader.seek(SeekFrom::Start((location.offset() * 4096) as u64))?;
            let result = RawChunk::parse(reader)?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub fn set_raw_chunk<W: Write + Read + Seek>(pos: ChunkPos, chunk: RawChunk, writer: &mut W) -> Result<()> {
        let mut chunk_data = Vec::new();
        chunk.write(&mut chunk_data)?;

        let sector_count = chunk_data.len() / 4096;

        writer.seek(SeekFrom::Start(0))?;
        let header = RegionHeader::parse(writer)?;

        let sector_offset = header.find_free_offset(pos, sector_count);

        let chunk_index = Region::get_chunk_index(pos);

        writer.seek(SeekFrom::Start(chunk_index as u64 * 4))?;
        writer.write_u24::<BigEndian>(sector_offset as u32)?;
        writer.write_u8(sector_count as u8)?;

        writer.seek(SeekFrom::Start((sector_offset * 4096) as u64))?;
        writer.write_all(&chunk_data)?;

        Ok(())
    }

    pub fn get_chunk_index(chunk: ChunkPos) -> usize {
        ((chunk.x & 31) as usize) + (((chunk.z & 31) as usize) * 32)
    }

    pub fn get_chunk_offset(index: usize) -> ChunkPos {
        ChunkPos {
            x: (index & 31) as i32,
            z: ((index / 32) & 31) as i32
        }
    }
    
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        let index = Region::get_chunk_index(pos);
        self.chunks.get(&index)
    }
    
    pub fn chunk_iter(&self) -> impl Iterator<Item=(ChunkPos, &Chunk)> {
        self.chunks.iter().map(|(index, chunk)| {
            (Self::get_chunk_offset(*index), chunk)
        })
    }
}
