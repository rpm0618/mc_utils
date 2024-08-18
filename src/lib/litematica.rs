use std::cmp::{max, min};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Error;
use quartz_nbt::{compound, io, NbtCompound, NbtList};
use quartz_nbt::io::Flavor;
use crate::positions::BlockPos;

// This whole file is a transliteration of LitematicStructureBuilder.java from earth's falling
// cluster finder https://github.com/Earthcomputer/FallingClusterFinderJava

// This struct specifically is ported from Litematica
struct LitematicaBitArray {
    long_array: Vec<i64>,
    bits_per_entry: i32,
    mask: i64,
    array_size: i64
}
impl LitematicaBitArray {
    pub fn new(bits_per_entry: i32, array_size: i64) -> LitematicaBitArray {
        let length = ((array_size * (bits_per_entry as i64) + 63) / 64) as usize;
        let mut backing_array: Vec<i64> = Vec::with_capacity(length);
        for _ in 0..length {
            backing_array.push(0);
        }
        LitematicaBitArray {
            long_array: backing_array,
            bits_per_entry,
            mask: (1 << bits_per_entry) - 1,
            array_size
        }
    }

    pub fn set_at(&mut self, index: i64, value: i32) {
        let start_offset = index * self.bits_per_entry as i64;
        let start_arr_index = (start_offset >> 6) as usize; // start_offset / 64
        let end_arr_index = (((index + 1) * (self.bits_per_entry as i64) - 1) >> 6) as usize;
        let start_bit_offset = (start_offset & 0x3F) as i32; // start_offset % 64

        self.long_array[start_arr_index] = self.long_array[start_arr_index] & !(self.mask << start_bit_offset) | (value as i64 & self.mask) << start_bit_offset;
        if start_arr_index != end_arr_index {
            let end_offset = 64 - start_bit_offset;
            let j1 = self.bits_per_entry - end_offset;
            self.long_array[end_arr_index] = (((self.long_array[end_arr_index] as u64) >> j1) as i64) << j1 | ((value as i64) & self.mask) >> end_offset;
        }
    }

    pub fn get_at(&self, index: i64) -> i32 {
        let start_offset = index * self.bits_per_entry as i64;
        let start_arr_index = (start_offset >> 6) as usize; // start_offset / 64
        let end_arr_index = (((index + 1) * (self.bits_per_entry as i64) - 1) >> 6) as usize;
        let start_bit_offset = (start_offset & 0x3F) as i32; // start_offset % 64

        if start_arr_index == end_arr_index {
            (self.long_array[start_arr_index] >> start_bit_offset) as i32
        } else {
            let end_offset = 64 - start_bit_offset;

            (((((self.long_array[start_arr_index] as u64) >> start_bit_offset) as i64) | self.long_array[end_arr_index] << end_offset) & self.mask) as i32
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
struct BlockState {
    block: String,
    properties: HashMap<String, String>
}
impl BlockState {
    pub fn new(block: String, properties: HashMap<String, String>) -> BlockState {
        BlockState {
            block, properties
        }
    }
}
impl Hash for BlockState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_str(&self.block);
        let mut keys: Vec<&String> = self.properties.keys().collect();
        keys.sort();
        for key in keys {
            state.write_str(key);
            state.write_str(&self.properties[key]);
        }
    }
}

pub struct LitematicaRegionBuilder {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    min_z: i32,
    max_z: i32,

    palette: HashMap<BlockState, i32>,
    storage: LitematicaBitArray
}
impl LitematicaRegionBuilder {
    pub fn new() -> LitematicaRegionBuilder {
        LitematicaRegionBuilder {
            min_x: 0,
            min_y: 0,
            min_z: 0,
            max_x: 0,
            max_y: 0,
            max_z: 0,

            palette: HashMap::new(),
            storage: LitematicaBitArray::new(2, 1)
        }
    }

    pub fn get_origin(&self) -> BlockPos {
        BlockPos {
            x: self.min_x,
            y: self.min_y,
            z: self.min_z
        }
    }

    pub fn set_block(&mut self, pos: BlockPos, block: String, properties: HashMap<String, String>) {
        let block = if block.contains(":") { block } else {
            format!("minecraft:{}", block)
        };

        if self.palette.is_empty() {
            self.min_x = pos.x;
            self.min_y = pos.y;
            self.min_z = pos.z;
            self.max_x = pos.x;
            self.max_y = pos.y;
            self.max_z = pos.z;
            self.palette.insert(BlockState::new("minecraft:air".to_owned(), HashMap::new()), 0);
        } else {
            let min_x = min(self.min_x, pos.x);
            let min_y = min(self.min_y, pos.y);
            let min_z = min(self.min_z, pos.z);
            let max_x = max(self.max_x, pos.x);
            let max_y = max(self.max_y, pos.y);
            let max_z = max(self.max_z, pos.z);

            // Resize backing array if bounding box has changed
            if min_x != self.min_x || min_y != self.min_y || min_z != self.min_z || max_x != self.max_x || max_y != self.max_y || max_z != self.max_z {
                let mut new_storage = LitematicaBitArray::new(self.storage.bits_per_entry, ((max_x - min_x + 1) * (max_y - min_y + 1) * (max_z - min_z + 1)) as i64);
                for new_x in min_x..=max_x {
                    for new_y in min_y..=max_y {
                        for new_z in min_z..=max_z {
                            let old_value = if new_x >= self.min_x && new_x <= self.max_x && new_y >= self.min_y && new_y <= self.max_y && new_z >= self.min_z && new_z <= self.max_z {
                                let x_size = (self.max_x - self.min_x + 1) as i64;
                                let z_size = (self.max_z - self.min_z + 1) as i64;
                                self.storage.get_at(((new_y - self.min_y) as i64) * x_size * z_size + ((new_z - self.min_z) as i64) * x_size + ((new_x - self.min_x) as i64))
                            } else {
                                0
                            };
                            let x_size = (max_x - min_x + 1) as i64;
                            let z_size = (max_z - min_z + 1) as i64;
                            new_storage.set_at(((new_y - min_y) as i64) * x_size * z_size + ((new_z - min_z) as i64) * x_size + ((new_x - min_x) as i64), old_value);
                        }
                    }
                }
                self.min_x = min_x;
                self.min_y = min_y;
                self.min_z = min_z;
                self.max_x = max_x;
                self.max_y = max_y;
                self.max_z = max_z;
                self.storage = new_storage;
            }
        }

        let block_state = BlockState::new(block, properties.clone());
        let index = self.palette.get(&block_state);
        let index = if let None = index {
            let index = self.palette.len() as i32;
            self.palette.insert(block_state, index);
            index
        } else {
            *index.unwrap()
        };

        // Resize palette if needed
        if index & (index - 1) == 0 {
            let bits_required = self.palette.len().next_power_of_two().trailing_zeros() as i32;
            if bits_required > self.storage.bits_per_entry {
                let mut new_storage = LitematicaBitArray::new(bits_required, self.storage.array_size);
                for i in 0..self.storage.array_size {
                    new_storage.set_at(i, self.storage.get_at(i))
                }
                self.storage = new_storage;
            }
        }

        let x_size = (self.max_x - self.min_x + 1) as i64;
        let z_size = (self.max_z - self.min_z + 1) as i64;
        self.storage.set_at(((pos.y - self.min_y) as i64) * x_size * z_size + ((pos.z - self.min_z) as i64) * x_size + ((pos.x - self.min_x) as i64), index);
    }

    pub fn fill(&mut self, x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32, block: String, properties: HashMap<String, String>) {
        let min_x = min(x1, x2);
        let min_y = min(y1, y2);
        let min_z = min(z1, z2);
        let max_x = max(x1, x2);
        let max_y = max(y1, y2);
        let max_z = max(z1, z2);

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                for z in min_z..=max_z {
                    self.set_block((x, y, z).into(), block.clone(), properties.clone());
                }
            }
        }
    }

    fn to_nbt(&self, origin: BlockPos) -> NbtCompound {
        let enclosing_size = compound! {
            "x": self.max_x - self.min_x  + 1,
            "y": self.max_y - self.min_y  + 1,
            "z": self.max_z - self.min_z  + 1,
        };
        let mut block_state_palette = NbtList::new();
        for _ in 0..self.palette.len() {
            block_state_palette.push(NbtCompound::new());
        }
        for key in self.palette.keys() {
            let index = self.palette[key];
            let block_state: &mut NbtCompound = block_state_palette.get_mut(index as usize).unwrap();
            block_state.insert("Name", key.block.clone());
            let mut properties = NbtCompound::new();
            for property_key in key.properties.keys() {
                let property_value = key.properties[property_key].clone();
                properties.insert(property_key, property_value);
            }
            block_state.insert("Properties", properties);
        }
        let region = compound! {
            "Position": compound! { "x": self.min_x - origin.x, "y": self.min_y - origin.y, "z": self.min_z - origin.z },
            "Size": enclosing_size,
            "TileEntities": NbtList::new(),
            "Entities": NbtList::new(),
            "BlockStatePalette": NbtList::from(block_state_palette),
            "BlockStates": self.storage.long_array.clone()
        };

        region
    }

    fn total_blocks(&self) -> i32 {
        let mut total_blocks = 0;
        let x_size = (self.max_x - self.min_x + 1) as i64;
        let z_size = (self.max_z - self.min_z + 1) as i64;
        for x in self.min_x..=self.max_x {
            for y in self.min_y..=self.max_y {
                for z in self.min_z..=self.max_z {
                    let index = self.storage.get_at(((y - self.min_y) as i64) * x_size * z_size + ((z - self.min_z) as i64) * x_size + ((x - self.min_x) as i64));
                    if index != 0 {
                        total_blocks += 1;
                    }
                }
            }
        }
        total_blocks
    }
}

#[derive(Debug)]
struct Extents {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    min_z: i32,
    max_z: i32,
}

pub struct LitematicaBuilder {
    regions: HashMap<String, LitematicaRegionBuilder>
}

impl LitematicaBuilder {
    pub fn new() -> LitematicaBuilder {
        LitematicaBuilder {
            regions: HashMap::new()
        }
    }

    pub fn add_region(&mut self, name: &str, region: LitematicaRegionBuilder) {
        self.regions.insert(String::from(name), region);
    }

    fn get_extents(&self) -> Extents {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut min_z = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut max_z = i32::MIN;

        for (_, region) in &self.regions {
            min_x = min(min_x, region.min_x);
            min_y = min(min_y, region.min_y);
            min_z = min(min_z, region.min_z);
            max_x = max(max_x, region.max_x);
            max_y = max(max_y, region.max_y);
            max_z = max(max_z, region.max_z);
        }
        Extents {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z
        }
    }

    pub fn get_origin(&self) -> BlockPos {
        let extents = self.get_extents();
        BlockPos::new(extents.min_x, extents.min_y, extents.min_z)
    }

    pub fn save(&self, path: &str, name: &str) -> Result<(), Error> {

        let mut regions = NbtCompound::new();

        let mut total_blocks = 0;
        let mut total_volume = 0;

        let extents = self.get_extents();
        let origin = BlockPos::new(extents.min_x, extents.min_y, extents.min_z);

        for (name, region) in &self.regions {
            total_blocks += region.total_blocks();
            total_volume += ((region.max_x - region.min_x  + 1) as i64) * ((region.max_y - region.min_y  + 1) as i64) * ((region.max_z - region.min_z  + 1) as i64);
            regions.insert(name, region.to_nbt(origin));
        }

        let enclosing_size = compound! {
            "x": extents.max_x - extents.min_x + 1,
            "y": extents.max_y - extents.min_y + 1,
            "z": extents.max_z - extents.min_z + 1,
        };
        let metadata = compound! {
            "TimeCreated": 10101010101i64,
            "TimeModified": 10101010101i64,
            "EnclosingSize": enclosing_size.clone(),
            "Description": "",
            "RegionCount": regions.len() as i32,
            "TotalBlocks": total_blocks,
            "Author": "rpm0618",
            "TotalVolume": total_volume,
            "Name": name
        };

        let root_tag = compound! {
            "MinecraftDataVersion": 1343,
            "Version": 4,
            "Metadata": metadata,
            "Regions": regions
        };

        let mut data: Vec<u8> = Vec::new();
        io::write_nbt(&mut data, None, &root_tag, Flavor::GzCompressed).expect("NBT Write Failed");
        std::fs::write(path, &data)
    }
}