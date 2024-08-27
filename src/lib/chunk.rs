use std::io::{Error, Read};
use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::{GzDecoder, ZlibDecoder};
use java_string::JavaString;
use crate::positions::{BlockPos, ChunkPos};
use crate::block::Block;
use crate::nbt;
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
    fn visit_level(&mut self, val: LeafTag, path: &NbtPath) -> nbt::Result<()> {
        let Some(NbtPathElement::Element(second_level)) = path.get(2) else {
            return Err(nbt::NbtError::Custom("Unexpected Chunk Structure".to_string()));
        };
        if second_level == "Sections" {
            self.visit_section(val, path)?;
        } else if second_level == "Entities" {
            self.visit_entities(val, path)?;
        } else if second_level == "xPos" {
            let LeafTag::Int(x_pos) = val else {
                return Err(nbt::NbtError::Custom("Unexpected Chunk Structure, chunk xPos is not an int".to_string()));
            };
            self.data.pos.x = x_pos;
        } else if second_level == "zPos" {
            let LeafTag::Int(z_pos) = val else {
                return Err(nbt::NbtError::Custom("Unexpected Chunk Structure, chunk zPos is not an int".to_string()));
            };
            self.data.pos.z = z_pos;
        }

        Ok(())
    }

    #[inline]
    fn visit_section(&mut self, val: LeafTag, path: &NbtPath) -> nbt::Result<()> {
        let Some(NbtPathElement::Index(index)) = path.get(3) else {
            return Err(nbt::NbtError::Custom("Unexpected Chunk Structure".to_string()));
        };
        let index = *index;
        if index == self.data.sections.len() {
            self.data.sections.push(ChunkSection {
                blocks: Vec::new(),
                block_data: Vec::new()
            });
        }
        if index >= self.data.sections.len() {
            return Err(nbt::NbtError::Custom("Unexpected Section Visitor Order".to_string()));
        }
        let curr_section = self.data.sections.get_mut(index).unwrap();
        let Some(NbtPathElement::Element(field_name)) = path.get(4) else {
            return Err(nbt::NbtError::Custom("Unexpected Chunk Structure".to_string()));
        };

        if field_name == "Blocks" {
            let LeafTag::ByteArray(blocks) = val else {
                return Err(nbt::NbtError::Custom("Unexpected Chunk Structure, Blocks not a byte array".to_string()));
            };
            curr_section.blocks = blocks;
        } else if field_name == "Data" {
            let LeafTag::ByteArray(block_data) = val else {
                return Err(nbt::NbtError::Custom("Unexpected Chunk Structure, Data not a byte array".to_string()));
            };
            curr_section.block_data = block_data;
        }

         Ok(())
    }

    #[inline]
    fn visit_entities(&mut self, val: LeafTag, path: &NbtPath) -> nbt::Result<()> {
        let Some(NbtPathElement::Index(index)) = path.get(3) else {
            return Err(nbt::NbtError::Custom("Unexpected Chunk Structure".to_string()));
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
            return Err(nbt::NbtError::Custom("Unexpected Entity Visitor Order".to_string()));
        }
        let curr_entity = self.data.entities.get_mut(index).unwrap();
        let Some(NbtPathElement::Element(field_name)) = path.get(4) else {
            return Err(nbt::NbtError::Custom("Unexpected Chunk Structure".to_string()));
        };

        if *field_name == JavaString::from("id") {
            let LeafTag::String(id) = val else {
                return Err(nbt::NbtError::Custom("Unexpected Chunk Structure, entity id not string".to_string()));
            };
            curr_entity.id = id.into_string()?;
        } else if *field_name == JavaString::from("Block") {
            let LeafTag::String(block) = val else {
                return Err(nbt::NbtError::Custom("Unexpected Chunk Structure, entity Block not string".to_string()));
            };
            curr_entity.block = Some(block.into_string()?);
        }

        Ok(())
    }
}
impl NbtVisitor for ChunkVisitor {
    #[inline]
    fn visit_leaf(&mut self, val: LeafTag, path: &NbtPath) -> nbt::Result<()> {
        let Some(NbtPathElement::Element(first_level)) = path.get(1) else {
            return Err(nbt::NbtError::Custom("Unexpected Chunk Structure".to_string()));
        };
        if first_level == "Level" {
            self.visit_level(val, path)?;
        }
        Ok(())
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
