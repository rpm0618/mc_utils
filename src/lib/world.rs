use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Error, Read, Seek, SeekFrom};
use std::path::Path;
use quartz_nbt::io::Flavor;
use quartz_nbt::NbtCompound;
use crate::chunk_pos::ChunkPos;

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
}

pub struct Region {
    header: RegionHeader,
    chunks: HashMap<usize, Chunk>
}
impl Region {
    pub fn parse<R: Read + Seek>(reader: &mut R) -> Result<Region, Error> {
        let header = RegionHeader::parse(reader)?;
        let mut chunks: HashMap<usize, Chunk> = HashMap::new();

        for x in 0..32 {
            for z in 0..32 {
                let index = Region::get_chunk_index(x, z);
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

    fn get_chunk_index(x: i32, z: i32) -> usize {
        ((x & 31) as usize) + (((z & 31) as usize) * 32)
    }

    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        let index = Region::get_chunk_index(pos.x, pos.z);
        self.chunks.get(&index)
    }
}

pub enum Dimension {
    Overworld,
    Nether,
    End
}

pub struct World {
     regions: HashMap<(i32, i32), Region>,
     world_path: String
}
impl World {
    pub fn new(world_path: &str) -> World {
        World {
            regions: HashMap::new(),
            world_path: world_path.to_owned()
        }
    }

    pub fn get_chunk(&mut self, pos: ChunkPos, dim: Dimension) -> Result<Option<&Chunk>, Error> {
        let region_x = pos.x >> 5;
        let region_z = pos.z >> 5;

        if self.regions.contains_key(&(region_x, region_z)) {
            return Ok(self.regions[&(region_x, region_z)].get_chunk(pos));
        }

        let suffix = match dim {
            Dimension::Overworld => Path::new("region").into(),
            Dimension::Nether => Path::new("DIM-1").join("region"),
            Dimension::End => Path::new("DIM1").join("region")
        };
        let region_name = format!("r.{}.{}.mca", region_x, region_z);
        let path = Path::new(&self.world_path).join(&suffix).join(&region_name);

        if path.exists() {
            println!("Loading region {}", region_name);
            let region_data = std::fs::read(&path)?;
            let region = Region::parse(&mut Cursor::new(region_data))?;

            self.regions.insert((region_x, region_z), region);

            Ok(self.regions[&(region_x, region_z)].get_chunk(pos))
        } else {
            Ok(None)
        }
    }
}