#[derive(Copy, Clone)]
pub struct Block {
    pub block_id: u8,
    pub data: u8
}

impl Block {
    pub fn new(block_id: u8, data: u8) -> Block {
        Block {
            block_id,
            data
        }
    }
}