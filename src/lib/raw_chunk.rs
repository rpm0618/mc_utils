use std::io;
use std::io::{Cursor, Read, Write};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flate2::read::{GzDecoder, GzEncoder, ZlibDecoder};
use java_string::JavaString;
use valence_nbt::{compound, Compound, Value};
use valence_nbt::value::{ValueMut, ValueRef};
use crate::block::Block;
use crate::error::{McUtilsError, Result};
use crate::positions::BlockPos;

// RawChunk holds and operates on the entire NBT data of the chunk, allowing for round trips with
// modifications (currently only setting block states)
// https://minecraft.wiki/w/Chunk_format/Anvil/Before_1.13
// https://minecraft.wiki/w/Region_file_format
pub struct RawChunk {
    data: Compound<JavaString>
}
impl RawChunk {
    pub fn new(data: Compound<JavaString>) -> RawChunk {
        RawChunk { data }
    }

    pub fn parse<R: Read>(reader: &mut R) -> Result<RawChunk> {
        let length = reader.read_u32::<BigEndian>()?;
        let compression_type = reader.read_u8()?;

        let mut bytes = Vec::new();

        match compression_type {
            1 => {
                let mut reader = GzDecoder::new(reader);
                reader.read_to_end(&mut bytes)?;
            }
            2 => {
                let mut reader = ZlibDecoder::new(reader);
                reader.read_to_end(&mut bytes)?;
            }
            3 => {
                bytes.resize((length - 1) as usize, 0);
                reader.read_exact(&mut bytes)?;
            }
            _ => return Err(McUtilsError::UnknownCompressionType(compression_type))
        }

        let data = valence_nbt::from_binary::<JavaString>(&mut bytes.as_slice()).unwrap();
        Ok(RawChunk::new(data.0))
    }

    // Write chunk data (the entire contents of the sectors it should occupy in the region file,
    // including the length, compression type, and padding bytes at the end)
    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut bytes = Vec::new();
        valence_nbt::to_binary(&self.data, &mut bytes, "").unwrap();
        let cursor = Cursor::new(bytes);

        let mut compressor = GzEncoder::new(cursor, flate2::Compression::default());
        let mut out_data = Vec::new();
        io::copy(&mut compressor, &mut out_data)?;

        let len = out_data.len();
        let pad_len = (len + 5).next_multiple_of(4096) - (len + 5);

        writer.write_u32::<BigEndian>(len as u32)?;
        writer.write_u8(1)?; // Gzip compression type
        writer.write_all(&out_data)?;
        writer.write_all(&vec![0; pad_len])?;

        Ok(())
    }

    pub fn get_block_at(&self, pos: BlockPos)-> Result<Block> {
        let subchunk = (pos.y & 0xff) >> 4;

        let Some(Value::Compound(level)) = self.data.get("Level") else {
            return Err(McUtilsError::InvalidChunkFormat("Missing 'Level' compound in chunk data".to_string()));
        };
        let Some(Value::List(sections)) = level.get("Sections") else {
            return Err(McUtilsError::InvalidChunkFormat("Missing 'Sections' list in chunk data".to_string()));
        };

        for section in sections {
            let ValueRef::Compound(section_data) = section else {
                return Err(McUtilsError::InvalidChunkFormat("Invalid section in chunk data".to_string()));
            };
            let Some(Value::Byte(section_y)) = section_data.get("Y") else {
                return Err(McUtilsError::InvalidChunkFormat("Missing 'Y' byte in section".to_string()));
            };

            if *section_y != subchunk as i8 {
                continue;
            }

            let Some(Value::ByteArray(blocks)) = section_data.get("Blocks") else {
                return Err(McUtilsError::InvalidChunkFormat("Missing 'Blocks' byte array in section".to_string()));
            };

            let Some(Value::ByteArray(block_data)) = section_data.get("Data") else {
                return Err(McUtilsError::InvalidChunkFormat("Missing 'Data' byte array in section".to_string()));
            };

            let x = pos.x & 0xf;
            let y = pos.y & 0xf;
            let z = pos.z & 0xf;
            let index = (x | (y << 8) | (z << 4)) as usize;

            let block_id = blocks[index] as u8;
            let block_data_byte = block_data[index >> 1] as u8;
            let data = if index & 1 == 0 {
                block_data_byte & 0xf
            } else {
                block_data_byte >> 4 & 0xf
            };

            return Ok(Block::new(block_id, data));
        }

        Ok(Block::air())
    }

    pub fn set_block_at(&mut self, pos: BlockPos, block: Block) -> Result<()> {
        let subchunk = (pos.y & 0xff) >> 4;

        let Some(Value::Compound(level)) = self.data.get_mut("Level") else {
            return Err(McUtilsError::InvalidChunkFormat("Missing 'Level' compound in chunk data".to_string()));
        };
        let Some(Value::List(sections)) = level.get_mut("Sections") else {
            return Err(McUtilsError::InvalidChunkFormat("Missing 'Sections' list in chunk data".to_string()));
        };

        // Subchunk list is not always full, add empty subchunks as needed
        if subchunk >= sections.len() as i32 {
            let mut subchunk_index = sections.len() as i8;
            while sections.len() < subchunk as usize + 1 {
                let mut empty_section = compound! { <JavaString>
                    "Y" => subchunk_index,
                    "Blocks" => vec![0_i8; 4096],
                    "Data" => vec![0_i8; 2048],
                    "BlockLight" => vec![0_i8; 2048],
                    "SkyLight" => vec![-128_i8; 2048]
                };
                if !sections.try_push(ValueMut::Compound(&mut empty_section)) {
                    return Err(McUtilsError::InvalidChunkFormat("Sections is not a list of Compounds?".to_string()));
                }
                subchunk_index += 1;
            }
        }
        for section in sections {
            let ValueMut::Compound(section_data) = section else {
                return Err(McUtilsError::InvalidChunkFormat("Invalid section in chunk data".to_string()));
            };
            let Some(Value::Byte(section_y)) = section_data.get("Y") else {
                return Err(McUtilsError::InvalidChunkFormat("Missing 'Y' byte in section".to_string()));
            };

            if *section_y != subchunk as i8 {
                continue;
            }

            let x = pos.x & 0xf;
            let y = pos.y & 0xf;
            let z = pos.z & 0xf;
            let index = (x | (y << 8) | (z << 4)) as usize;

            {
                let Some(Value::ByteArray(blocks)) = section_data.get_mut("Blocks") else {
                    return Err(McUtilsError::InvalidChunkFormat("Missing 'Blocks' byte array in section".to_string()));
                };
                blocks[index] = block.block_id as i8;
            }
            {
                let Some(Value::ByteArray(block_data)) = section_data.get_mut("Data") else {
                    return Err(McUtilsError::InvalidChunkFormat("Missing 'Data' byte array in section".to_string()));
                };

                let block_data_byte = block_data[index >> 1] as u8;
                let data = if index & 1 == 0 {
                    (block_data_byte & 0xf0) | (block.data & 0xf)
                } else {
                    (block.data & 0xf) << 4 | (block_data_byte & 0xf)
                };

                block_data[index >> 1] = data as i8;
            }

            return Ok(());
        }

        Ok(())
    }

    pub fn set_block_with_te(&mut self, pos: BlockPos, block: Block, te_data: Compound<JavaString>) -> Result<()> {
        self.set_block_at(pos, block)?;

        let mut full_te_data = compound! { <JavaString>
            "x" => pos.x,
            "y" => pos.y,
            "z" => pos.z,
        };
        full_te_data.merge(te_data);

        let Some(Value::Compound(level)) = self.data.get_mut("Level") else {
            return Err(McUtilsError::InvalidChunkFormat("Missing 'Level' compound in chunk data".to_string()));
        };
        let Some(Value::List(tile_entities)) = level.get_mut("TileEntities") else {
            return Err(McUtilsError::InvalidChunkFormat("Missing 'TileEntities' list in chunk data".to_string()));
        };

        for te in tile_entities.iter_mut() {
            let ValueMut::Compound(te) = te else {
                return Err(McUtilsError::InvalidChunkFormat("Invalid tile entity in chunk data".to_string()));
            };
            let Some(Value::Int(x)) = te.get("x") else {
                return Err(McUtilsError::InvalidChunkFormat("Missing 'x' field in tile entity".to_string()));
            };
            let Some(Value::Int(y)) = te.get("y") else {
                return Err(McUtilsError::InvalidChunkFormat("Missing 'y' field in tile entity".to_string()));
            };
            let Some(Value::Int(z)) = te.get("z") else {
                return Err(McUtilsError::InvalidChunkFormat("Missing 'z' field in tile entity".to_string()));
            };

            if *x == pos.x && *y == pos.y && *z == pos.z {
                te.clear();
                te.merge(full_te_data);
                return Ok(());
            }
        }
        if !tile_entities.try_push(full_te_data) {
            return Err(McUtilsError::InvalidChunkFormat("Failed to push tile entity into chunk data".to_string()));
        }

        Ok(())
    }
}