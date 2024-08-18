mod error;

pub use error::{Error, Result};

use std::io::Read;
use byteorder::{BigEndian, ReadBytesExt};
use java_string::{JavaString};
use num_enum::{TryFromPrimitive};
use crate::nbt::NbtPathElement::{Element, Index};
use crate::util::{cast_byte_buf_to_signed, read_i32_array, read_i64_array};

#[derive(Debug)]
pub enum LeafTag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(JavaString),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

pub trait NbtVisitor {
    fn visit_leaf(&mut self, val: LeafTag, path: &NbtPath);
}

#[derive(Debug)]
pub enum NbtPathElement {
    Element(JavaString),
    Index(usize),
}

#[derive(Debug)]
pub struct NbtPath(Vec<NbtPathElement>);
impl NbtPath {
    fn new() -> Self {
        NbtPath(Vec::new())
    }

    fn push(&mut self, path_element: NbtPathElement) {
        self.0.push(path_element);
    }

    fn pop(&mut self) -> Option<NbtPathElement> {
        self.0.pop()
    }

    pub fn peek(&self,) -> Option<&NbtPathElement> {
        self.0.get(self.len() - 1)
    }

    pub fn peek_back(&self, back: usize) -> Option<&NbtPathElement> {
        self.0.get(self.len() - (1 + back))
    }

    pub fn get(&self, index: usize) -> Option<&NbtPathElement> {
        self.0.get(index)
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum TagId {
    End = 0x0,
    Byte = 0x1,
    Short = 0x2,
    Int = 0x3,
    Long = 0x4,
    Float = 0x5,
    Double = 0x6,
    ByteArray = 0x7,
    String = 0x8,
    List = 0x9,
    Compound = 0xA,
    IntArray = 0xB,
    LongArray = 0xC,
}

pub fn visit_nbt<R: Read, V: NbtVisitor>(reader: &mut R, visitor: &mut V) -> Result<()> {
    let root_id: TagId = reader.read_u8()?.try_into()?;
    if root_id != TagId::Compound {
        return Err(Error::InvalidNbtRoot(root_id));
    }

    let mut curr_path = NbtPath::new();

    let root_name = read_string(reader)?;
    curr_path.push(Element(root_name));

    visit_tag_body(reader, visitor, root_id, &mut curr_path)?;

    Ok(())
}

fn visit_tag_body<R: Read, V: NbtVisitor>(reader: &mut R, visitor: &mut V, tag_id: TagId, curr_path: &mut NbtPath) -> Result<()> {
    match tag_id {
        TagId::Byte => {
            visitor.visit_leaf(LeafTag::Byte(reader.read_i8()?), curr_path);
        }
        TagId::Short => {
            visitor.visit_leaf(LeafTag::Short(reader.read_i16::<BigEndian>()?), curr_path);
        }
        TagId::Int => {
            visitor.visit_leaf(LeafTag::Int(reader.read_i32::<BigEndian>()?), curr_path);
        }
        TagId::Long => {
            visitor.visit_leaf(LeafTag::Long(reader.read_i64::<BigEndian>()?), curr_path);
        }
        TagId::Float => {
            visitor.visit_leaf(LeafTag::Float(reader.read_f32::<BigEndian>()?), curr_path);
        }
        TagId::Double => {
            visitor.visit_leaf(LeafTag::Double(reader.read_f64::<BigEndian>()?), curr_path);
        }
        TagId::ByteArray => {
            let len = reader.read_i32::<BigEndian>()? as usize;
            let mut bytes = vec![0; len];
            reader.read_exact(&mut bytes)?;
            visitor.visit_leaf(LeafTag::ByteArray(cast_byte_buf_to_signed(bytes)), curr_path);
        }
        TagId::String => {
            visitor.visit_leaf(LeafTag::String(read_string(reader)?), curr_path);
        }
        TagId::List => {
            let tag_id = reader.read_u8()?.try_into()?;
            let len = reader.read_i32::<BigEndian>()? as usize;

            for i in 0..len {
                curr_path.push(Index(i));
                visit_tag_body(reader, visitor, tag_id, curr_path)?;
                curr_path.pop();
            }
        }
        TagId::Compound => {
            let mut tag_id = reader.read_u8()?.try_into()?;
            while tag_id != TagId::End {
                let name = read_string(reader)?;
                curr_path.push(Element(name));
                visit_tag_body(reader, visitor, tag_id, curr_path)?;
                curr_path.pop();
                tag_id = reader.read_u8()?.try_into()?;
            }
        }
        TagId::IntArray => {
            let len = reader.read_i32::<BigEndian>()? as usize;
            visitor.visit_leaf(LeafTag::IntArray(read_i32_array(reader, len)?), curr_path);
        }
        TagId::LongArray => {
            let len = reader.read_i32::<BigEndian>()? as usize;
            visitor.visit_leaf(LeafTag::LongArray(read_i64_array(reader, len)?), curr_path);
        }
        TagId::End => return Err(Error::InvalidNbtEndTag)
    }

    Ok(())
}

#[inline]
fn read_string<R: Read>(reader: &mut R) -> Result<JavaString> {
    let len = reader.read_u16::<BigEndian>()? as usize;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;

    Ok(JavaString::from_modified_utf8(bytes)?)
}
