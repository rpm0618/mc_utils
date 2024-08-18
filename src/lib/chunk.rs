use std::io::{Error, Read};
use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::{GzDecoder, ZlibDecoder};
use java_string::JavaString;
use crate::positions::{BlockPos, ChunkPos};
use crate::block::Block;
use crate::nbt::{LeafTag, NbtPath, NbtPathElement, NbtVisitor, visit_nbt};

pub struct Chunk {
    pub length: u32,
    pub compression_type: u8,
    data: ChunkData
}

#[derive(Debug)]
struct ChunkData {
    pos: ChunkPos,
    sections: Vec<ChunkSection>,
    entities: Vec<Entity>
}
#[derive(Debug)]
struct ChunkSection {
    blocks: Vec<i8>,
    block_data: Vec<i8>
}

#[derive(Debug)]
pub struct Entity {
    pub pos: (f64, f64, f64),
    pub id: String,
    pub block: Option<String>
}

struct ChunkVisitor {
    data: ChunkData
}
impl ChunkVisitor {
    fn new() -> ChunkVisitor {
        ChunkVisitor {
            data: ChunkData {
                pos: (0, 0).into(),
                sections: Vec::new(),
                entities: Vec::new()
            }
        }
    }

    #[inline]
    fn visit_level(&mut self, val: LeafTag, path: &NbtPath) {
        let Some(NbtPathElement::Element(second_level)) = path.get(2) else {
            panic!("Unexpected Chunk Structure");
        };
        if *second_level == JavaString::from("Sections") {
            self.visit_section(val, path);
        } else if *second_level == JavaString::from("Entities") {
            self.visit_entities(val, path);
        } else if *second_level == JavaString::from("xPos") {
            let LeafTag::Int(x_pos) = val else {
                panic!("Unexpected Chunk Structure");
            };
            self.data.pos.x = x_pos;
        } else if *second_level == JavaString::from("zPos") {
            let LeafTag::Int(z_pos) = val else {
                panic!("Unexpected Chunk Structure");
            };
            self.data.pos.z = z_pos;
        }
    }

    #[inline]
    fn visit_section(&mut self, val: LeafTag, path: &NbtPath) {
        let Some(NbtPathElement::Index(index)) = path.get(3) else {
            panic!("Unexpected Chunk Structure");
        };
        let index = *index;
        if index == self.data.sections.len() {
            self.data.sections.push(ChunkSection {
                blocks: Vec::new(),
                block_data: Vec::new()
            });
        }
        if index >= self.data.sections.len() {
            panic!("Unexpected Visitor Order");
        }
        let curr_section = self.data.sections.get_mut(index).unwrap();
        let Some(NbtPathElement::Element(field_name)) = path.get(4) else {
            panic!("Unexpected Chunk Structure");
        };

        if *field_name == JavaString::from("Blocks") {
            let LeafTag::ByteArray(blocks) = val else {
                panic!("Unexpected Chunk Structure");
            };
            curr_section.blocks = blocks;
        } else if *field_name == JavaString::from("Data") {
            let LeafTag::ByteArray(block_data) = val else {
                panic!("Unexpected Chunk Structure");
            };
            curr_section.block_data = block_data;
        }
    }

    #[inline]
    fn visit_entities(&mut self, val: LeafTag, path: &NbtPath) {
        let Some(NbtPathElement::Index(index)) = path.get(3) else {
            panic!("Unexpected Chunk Structure");
        };
        let index = *index;
        if index == self.data.entities.len() {
            self.data.entities.push(Entity {
                id: "".into(),
                pos: (0.0, 0.0, 0.0),
                block: None
            });
        }
        if index >= self.data.entities.len() {
            panic!("Unexpected Visitor Order");
        }
        let curr_entity = self.data.entities.get_mut(index).unwrap();
        let Some(NbtPathElement::Element(field_name)) = path.get(4) else {
            panic!("Unexpected Chunk Structure");
        };

        if *field_name == JavaString::from("id") {
            let LeafTag::String(id) = val else {
                panic!("Unexpected Chunk Structure");
            };
            curr_entity.id = id.into_string().unwrap();
        } else if *field_name == JavaString::from("Block") {
            let LeafTag::String(block) = val else {
                panic!("Unexpected Chunk Structure");
            };
            curr_entity.block = Some(block.into_string().unwrap());
        }
    }
}
impl NbtVisitor for ChunkVisitor {
    #[inline]
    fn visit_leaf(&mut self, val: LeafTag, path: &NbtPath) {
        let Some(NbtPathElement::Element(first_level)) = path.get(1) else {
            panic!("Unexpected Chunk Structure");
        };
        if *first_level == JavaString::from("Level") {
            self.visit_level(val, path);
        }
    }
}

impl Chunk {
    pub fn parse<R: Read>(reader: &mut R) -> Result<Chunk, Error> {
        let length = reader.read_u32::<BigEndian>()?;
        let compression_type = reader.read_u8()?;

        let mut visitor = ChunkVisitor::new();
        match compression_type {
            1 => {
                let mut reader = GzDecoder::new(reader);
                visit_nbt(&mut reader, &mut visitor).unwrap();
            }
            2 => {
                let mut reader = ZlibDecoder::new(reader);
                visit_nbt(&mut reader, &mut visitor).unwrap();
            }
            3 => {
                visit_nbt(reader, &mut visitor).unwrap();
            }
            _ => panic!("Unknown compression type {}", compression_type)
        };

        Ok(Chunk {
            length,
            compression_type,
            data: visitor.data
        })
    }

    pub fn block_at(&self, pos: BlockPos) -> u8 {
        let subchunk = (pos.y >> 4) as usize;
        if let Some(section) = self.data.sections.get(subchunk) {
            let blocks = &section.blocks;
            let x = pos.x & 0xf;
            let y = pos.y & 0xf;
            let z = pos.z & 0xf;
            let index = (x | (y << 8) | (z << 4)) as usize;
            blocks[index] as u8
        } else {
            0
        }
    }

    pub fn block_iter(&self) -> impl Iterator<Item=(BlockPos, Block)> + '_ {
        self.data.sections.iter().enumerate().flat_map(|(subchunk, section)| {
            (0..4096).map(move |index| {
                let block_id = section.blocks[index] as u8;
                let block_data_byte = section.block_data[index >> 1] as u8;
                let data = if index & 1 == 0 {
                    block_data_byte & 0xf
                } else {
                    block_data_byte >> 4 & 0xf
                };

                let block_x = (index & 0xf) as i32;
                let block_y = (subchunk as i32) << 4 | ((index >> 8) & 0xf) as i32;
                let block_z = ((index >> 4) & 0xf) as i32;

                ((block_x, block_y, block_z).into(), Block::new(block_id, data))
            })
        })
    }

    pub fn entity_iter(&self) -> impl Iterator<Item=&Entity> + '_ {
        self.data.entities.iter()
    }
}
