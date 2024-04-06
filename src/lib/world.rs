use std::collections::HashMap;
use std::fs::{OpenOptions};
use std::io::{Cursor, Error, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use crate::chunk::Chunk;
use crate::positions::{ChunkPos, RegionPos};
use crate::region::Region;

// https://minecraft.fandom.com/wiki/Region_file_format

pub enum Dimension {
    Overworld,
    Nether,
    End
}

pub struct World {
    overworld_regions: HashMap<RegionPos, Region>,
    nether_regions: HashMap<RegionPos, Region>,
    end_regions: HashMap<RegionPos, Region>,
    pub world_path: String
}
impl World {
    pub fn new(world_path: &str) -> World {
        World {
            overworld_regions: HashMap::new(),
            nether_regions: HashMap::new(),
            end_regions: HashMap::new(),
            world_path: world_path.to_owned()
        }
    }

    pub fn get_chunk(&mut self, pos: ChunkPos, dim: Dimension) -> Result<Option<&Chunk>, Error> {
        let region_x = pos.x >> 5;
        let region_z = pos.z >> 5;
        let region_pos = RegionPos::new(region_x, region_z);

        Ok(self.get_region(region_pos, dim)?.and_then(|r| r.get_chunk(pos)))
    }

    pub fn delete_chunk(&self, pos: ChunkPos, dim: Dimension) -> Result<(), Error> {
        let path = World::get_region_path(&self.world_path, pos.into(), dim);

        if path.exists() {
            // Overwrite the chunk's location entry in the region header
            let mut region_file = OpenOptions::new().read(true).write(true).open(path)?;
            let offset = Region::get_chunk_index(pos) * 4;
            region_file.seek(SeekFrom::Start(offset as u64))?;
            let data = [0u8; 4];
            region_file.write_all(&data)?;
        }
        Ok(())
    }

    pub fn get_region(&mut self, pos: RegionPos, dim: Dimension) -> Result<Option<&Region>, Error> {
        let regions = match dim {
            Dimension::Overworld => &mut self.overworld_regions,
            Dimension::Nether => &mut self.nether_regions,
            Dimension::End => &mut self.end_regions
        };
        if regions.contains_key(&pos) {
            return Ok(regions.get(&pos));
        }

        let path = World::get_region_path(&self.world_path, pos, dim);

        if path.exists() {
            println!("Loading region {:?}", path);
            let region_data = std::fs::read(&path)?;
            let region = Region::parse(&mut Cursor::new(region_data))?;

            regions.insert(pos, region);

            Ok(Some(&regions[&pos]))
        } else {
            Ok(None)
        }
    }

    pub fn get_region_path(world_path: &str, pos: RegionPos, dim: Dimension) -> PathBuf {
        let suffix = match dim {
            Dimension::Overworld => Path::new("region").into(),
            Dimension::Nether => Path::new("DIM-1").join("region"),
            Dimension::End => Path::new("DIM1").join("region")
        };
        let region_name = format!("r.{}.{}.mca", pos.x, pos.z);
        Path::new(world_path).join(&suffix).join(&region_name)
    }
}