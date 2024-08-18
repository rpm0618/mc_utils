use std::io::{Error, Read};
use std::mem::ManuallyDrop;
use std::ptr;

// Utility methods stolen from quartz_nbt
#[inline]
pub fn cast_byte_buf_to_signed(buf: Vec<u8>) -> Vec<i8> {
    let mut me = ManuallyDrop::new(buf);
    // Pointer cast is valid because i8 and u8 have the same layout
    let ptr = me.as_mut_ptr() as *mut i8;
    let length = me.len();
    let capacity = me.capacity();

    // Safety
    // * `ptr` was allocated by a Vec
    // * i8 has the same size and alignment as u8
    // * `length` and `capacity` came from a valid Vec
    unsafe { Vec::from_raw_parts(ptr, length, capacity) }
}

#[inline]
pub fn read_i32_array<R: Read>(reader: &mut R, len: usize) -> Result<Vec<i32>, Error> {
    let mut bytes = ManuallyDrop::new(vec![0i32; len]);

    let ptr = bytes.as_mut_ptr() as *mut u8;
    let length = bytes.len() * 4;
    let capacity = bytes.capacity() * 4;

    let mut bytes = unsafe { Vec::from_raw_parts(ptr, length, capacity) };

    reader.read_exact(&mut bytes)?;

    // Safety: the length of the vec is a multiple of 4, and the alignment is 4
    Ok(unsafe { convert_be_int_array_in_place::<i32, 4>(bytes, i32::from_be_bytes) })
}

#[inline]
pub fn read_i64_array<R: Read>(reader: &mut R, len: usize) -> Result<Vec<i64>, Error> {
    let mut bytes = ManuallyDrop::new(vec![0i64; len]);

    let ptr = bytes.as_mut_ptr() as *mut u8;
    let length = bytes.len() * 8;
    let capacity = bytes.capacity() * 8;

    let mut bytes = unsafe { Vec::from_raw_parts(ptr, length, capacity) };

    reader.read_exact(&mut bytes)?;

    // Safety: the length of the vec is a multiple of 8, and the alignment is 8
    Ok(unsafe { convert_be_int_array_in_place::<i64, 8>(bytes, i64::from_be_bytes) })
}

#[inline]
unsafe fn convert_be_int_array_in_place<I, const SIZE: usize>(
    mut bytes: Vec<u8>,
    convert: fn([u8; SIZE]) -> I,
) -> Vec<I> {
    let mut buf: [u8; SIZE];

    let mut read = bytes.as_ptr() as *const [u8; SIZE];
    let mut write = bytes.as_mut_ptr() as *mut I;
    let end = bytes.as_ptr().add(bytes.len()) as *const [u8; SIZE];

    while read != end {
        buf = ptr::read(read);
        ptr::write(write, convert(buf));
        read = read.add(1);
        write = write.add(1);
    }

    let mut me = ManuallyDrop::new(bytes);

    let ptr = me.as_mut_ptr() as *mut I;
    let length = me.len();
    let capacity = me.capacity();

    Vec::from_raw_parts(ptr, length / SIZE, capacity / SIZE)
}