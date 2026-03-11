use std::collections::HashMap;
use std::fs::{DirEntry, OpenOptions, read_dir};
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use java_string::JavaString;
use valence_nbt::Compound;
use crate::block::Block;
use crate::chunk::Chunk;
use crate::error::{McUtilsError, Result};
use crate::level_dat::LevelDat;
use crate::positions::{BlockPos, ChunkPos, RegionPos};
use crate::raw_chunk::RawChunk;
use crate::region::Region;

// https://minecraft.fandom.com/wiki/Region_file_format

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
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

    pub fn get_chunk(&mut self, pos: ChunkPos, dim: Dimension) -> Result<Option<&Chunk>> {
        let region_x = pos.x >> 5;
        let region_z = pos.z >> 5;
        let region_pos = RegionPos::new(region_x, region_z);

        Ok(self.get_region(region_pos, dim)?.and_then(|r| r.get_chunk(pos)))
    }

    pub fn get_raw_chunk_uncached(&self, pos: ChunkPos, dim: Dimension) -> Result<Option<RawChunk>> {
        let region_x = pos.x >> 5;
        let region_z = pos.z >> 5;
        let region_pos = RegionPos::new(region_x, region_z);

        let path = World::get_region_path(&self.world_path, region_pos, dim);

        if path.exists() {
            let region_data = std::fs::read(&path)?;
            let raw_chunk = Region::get_raw_chunk(pos, &mut Cursor::new(region_data))?;

            Ok(raw_chunk)
        } else {
            Ok(None)
        }
    }

    pub fn delete_chunk(&self, pos: ChunkPos, dim: Dimension) -> Result<()> {
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

    pub fn get_region(&mut self, pos: RegionPos, dim: Dimension) -> Result<Option<&Region>> {
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
            let region_data = std::fs::read(&path)?;
            let region = Region::parse(&mut Cursor::new(region_data))?;

            regions.insert(pos, region);
            Ok(Some(&regions[&pos]))
        } else {
            Ok(None)
        }
    }

    pub fn get_region_uncached(&self, pos: RegionPos, dim: Dimension) -> Result<Option<Region>> {
        let path = World::get_region_path(&self.world_path, pos, dim);
        if path.exists() {
            let region_data = std::fs::read(&path)?;
            let region = Region::parse(&mut Cursor::new(region_data))?;

            Ok(Some(region))
        } else {
            Ok(None)
        }
    }

    pub fn region_pos_iter(&self, dim: Dimension) -> Result<impl Iterator<Item=RegionPos> + '_> {
        Ok(Self::region_file_iter(&self.world_path, dim)?.map(move |res| {
            let entry = res.unwrap();
            let file_name = entry.file_name().into_string().unwrap();
            let parts: Vec<_> = file_name.split(".").collect();
            let x = parts[1].parse::<i32>().unwrap();
            let z = parts[2].parse::<i32>().unwrap();
            RegionPos::new(x, z)
        }))
    }

    pub fn get_num_regions(&self, dim: Dimension) -> Result<usize> {
        Ok(Self::region_file_iter(&self.world_path, dim)?.collect::<Vec<_>>().len())
    }

    pub fn get_level_dat(&self) -> Result<LevelDat> {
        let path = Path::new(&self.world_path).join("level.dat");
        let level_dat = std::fs::read(path)?;
        Ok(LevelDat::parse(&mut Cursor::new(level_dat))?)
    }

    pub fn get_spawn_chunks(&self) -> Result<impl Iterator<Item=ChunkPos> + '_> {
        let level_dat = self.get_level_dat()?;
        let spawn_point = level_dat.spawn;
        let spawn_chunk: ChunkPos = spawn_point.into();
        Ok(
            (-8..=8).flat_map(move |x| {
                (-8..=8).map(move |z| {
                    ChunkPos::new(spawn_chunk.x + x, spawn_chunk.z + z)
                }).filter(move |chunk_pos: &ChunkPos| {
                    let dx = chunk_pos.x * 16 + 8 - spawn_point.x;
                    let dz = chunk_pos.z * 16 + 8 - spawn_point.z;
                    dx >= -128 && dx <= 128 && dz >= -128 && dz <= 128
                })
            })
        )
    }

    pub fn set_block(&self, pos: BlockPos, dim: Dimension, block: Block) -> Result<()> {
        let region_pos: RegionPos = pos.into();
        let chunk_pos: ChunkPos = pos.into();

        let Some(mut raw_chunk) = self.get_raw_chunk_uncached(chunk_pos, dim)? else {
            return Err(McUtilsError::MissingChunk(chunk_pos, dim));
        };

        raw_chunk.set_block_at(pos, block)?;

        let path = World::get_region_path(&self.world_path, region_pos, dim);

        if !path.exists() {
            return Err(McUtilsError::MissingRegion(region_pos, dim));
        }

        let mut region_file = OpenOptions::new().read(true).write(true).open(path)?;
        Region::set_raw_chunk(chunk_pos, raw_chunk, &mut region_file)?;

        Ok(())
    }

    pub fn set_block_with_te(&self, pos: BlockPos, dim: Dimension, block: Block, te_data: Compound<JavaString>) -> Result<()> {
        let region_pos: RegionPos = pos.into();
        let chunk_pos: ChunkPos = pos.into();

        let Some(mut raw_chunk) = self.get_raw_chunk_uncached(chunk_pos, dim)? else {
            return Err(McUtilsError::MissingChunk(chunk_pos, dim));
        };

        raw_chunk.set_block_with_te(pos, block, te_data)?;

        let path = World::get_region_path(&self.world_path, region_pos, dim);

        if !path.exists() {
            return Err(McUtilsError::MissingRegion(region_pos, dim));
        }

        let mut region_file = OpenOptions::new().read(true).write(true).open(path)?;
        Region::set_raw_chunk(chunk_pos, raw_chunk, &mut region_file)?;

        Ok(())
    }

    fn region_file_iter(world_path: &str, dim: Dimension) -> Result<impl Iterator<Item=std::io::Result<DirEntry>>> {
        Ok(read_dir(Path::new(world_path).join(&Self::get_region_suffix(dim)))?.filter(|res| {
            if res.is_err() {
                return false;
            }
            let entry = res.as_ref().unwrap();
            entry.file_name().into_string().map_or(false, |name| name.contains("r.") && name.contains(".mca"))
        }))
    }

    fn get_region_path(world_path: &str, pos: RegionPos, dim: Dimension) -> PathBuf {
        let region_name = format!("r.{}.{}.mca", pos.x, pos.z);
        Path::new(world_path).join(&Self::get_region_suffix(dim)).join(&region_name)
    }

    fn get_region_suffix(dim: Dimension) -> PathBuf {
        match dim {
            Dimension::Overworld => Path::new("region").into(),
            Dimension::Nether => Path::new("DIM-1").join("region"),
            Dimension::End => Path::new("DIM1").join("region")
        }
    }
}