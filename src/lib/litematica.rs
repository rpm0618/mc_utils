use std::cmp::{max, min};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use quartz_nbt::{io, compound, NbtCompound, NbtList};
use quartz_nbt::io::Flavor;

// This whole file is a transliteration of LitematicStructureBuilder.java from earth's falling
// cluster finder https://github.com/Earthcomputer/FallingClusterFinderJava

// This struct specifically is ported from Litematica
struct LitematicaBitArray<> {
    long_array: Vec<i64>,
    bits_per_entry: i32,
    max_entry_value: i64,
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
            max_entry_value: (1 << bits_per_entry) - 1,
            array_size
        }
    }

    pub fn set_at(&mut self, index: i64, value: i32) {
        let start_offset = index * self.bits_per_entry as i64;
        let start_arr_index = (start_offset >> 6) as usize; // start_offset / 64
        let end_arr_index = (((index + 1) * (self.bits_per_entry as i64) - 1) >> 6) as usize;
        let start_bit_offset = (start_offset & 0x3F) as i32; // start_offset % 64

        self.long_array[start_arr_index] = self.long_array[start_arr_index] & !(self.max_entry_value << start_bit_offset) | (value as i64 & self.max_entry_value) << start_bit_offset;
        if start_arr_index != end_arr_index {
            let end_offset = 64 - start_bit_offset;
            let j1 = self.bits_per_entry - end_offset;
            self.long_array[end_arr_index] = (((self.long_array[end_arr_index] as u64) >> j1) as i64) << j1 | ((value as i64) & self.max_entry_value) >> end_offset;
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

            (((((self.long_array[start_arr_index] as u64) >> start_bit_offset) as i64) | self.long_array[end_arr_index] << end_offset) & self.max_entry_value) as i32
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

pub struct LitematicaBuilder {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    min_z: i32,
    max_z: i32,

    palette: HashMap<BlockState, i32>,
    storage: LitematicaBitArray
}
impl LitematicaBuilder {
    pub fn new() -> LitematicaBuilder {
        LitematicaBuilder {
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

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: String, properties: HashMap<String, String>) {
        let block = if block.contains(":") { block } else {
            let mut name = String::from("minecraft:");
            name.push_str(&block);
            name
        };

        if self.palette.is_empty() {
            self.min_x = x;
            self.min_y = y;
            self.min_z = z;
            self.max_x = x;
            self.max_y = y;
            self.max_z = z;
            self.palette.insert(BlockState::new("minecraft:air".to_owned(), HashMap::new()), 0);
        } else {
            let min_x = min(self.min_x, x);
            let min_y = min(self.min_y, y);
            let min_z = min(self.min_z, z);
            let max_x = max(self.max_x, x);
            let max_y = max(self.max_y, y);
            let max_z = max(self.max_z, z);

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
        self.storage.set_at(((y - self.min_y) as i64) * x_size * z_size + ((z - self.min_z) as i64) * x_size + ((x - self.min_x) as i64), index);
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
                    self.set_block(x, y, z, block.clone(), properties.clone());
                }
            }
        }
    }

    pub fn save(&self, path: &str) {
        let enclosing_size = compound! {
            "x": self.max_x - self.min_x  + 1,
            "y": self.max_y - self.min_y  + 1,
            "z": self.max_z - self.min_z  + 1,
        };
        let mut total_blocks = 0;
        for x in self.min_x..=self.max_x {
            for y in self.min_y..=self.max_y {
                for z in self.min_z..=self.max_z {
                    total_blocks += 1;
                }
            }
        }
        let metadata = compound! {
            "TimeCreated": 10101010101i64,
            "TimeModified": 10101010101i64,
            "EnclosingSize": enclosing_size.clone(),
            "Description": "Falling block cluster",
            "RegionCount": 1,
            "TotalBlocks": total_blocks,
            "Author": "rpm0618",
            "TotalVolume": ((self.max_x - self.min_x  + 1) as i64) * ((self.max_y - self.min_y  + 1) as i64) * ((self.max_z - self.min_z  + 1) as i64),
            "Name": "Cluster Chunks"
        };

        let mut block_state_palette = NbtList::new();
        for i in 0..self.palette.len() {
            block_state_palette.push(NbtCompound::new());
        }
        for key in self.palette.keys() {
            let index = self.palette[key];
            let mut block_state: &mut NbtCompound = block_state_palette.get_mut(index as usize).unwrap();
            block_state.insert("Name", key.block.clone());
            let mut properties = NbtCompound::new();
            for property_key in key.properties.keys() {
                let property_value = key.properties[property_key].clone();
                properties.insert(property_key, property_key);
            }
            block_state.insert("Properties", properties);
        }

        let region = compound! {
            "Position": compound! { "x": 0, "y": 0, "z": 0 },
            "Size": enclosing_size,
            "TileEntities": NbtList::new(),
            "Entities": NbtList::new(),
            "BlockStatePalette": NbtList::from(block_state_palette),
            "BlockStates": self.storage.long_array.clone()
        };
        let regions = compound! {
            "Cluster Chunks": region
        };

        let root_tag = compound! {
            "MinecraftDataVersion": 1343,
            "Version": 4,
            "Metadata": metadata,
            "Regions": regions
        };

        let mut data: Vec<u8> = Vec::new();
        io::write_nbt(&mut data, None, &root_tag, Flavor::GzCompressed).expect("NBT Write Failed");
        std::fs::write(path, &data).expect("File Write Failed");
    }
}