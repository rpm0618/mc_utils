fn xy_for(index: i32) -> (i32, i32) {
    (index % 16, index / 16)
}

fn main() {
    let palette_bits = 13;
    let mask = (1 << palette_bits) - 1;

    println!("mask: {:#b}", mask);

    for index in 0..256 {
        let i = index * palette_bits;
        let word_1 = i / 64;
        let word_2 = ((index + 1) * palette_bits - 1) / 64;
        

        if word_1 != word_2 {
            let (x, y) = xy_for(index);

            let offset_1 = 64 - (i % 64);
            let offset_2 = palette_bits - offset_1;
            
            println!("[{}] ({}, {}): Bits: {}/{}", index, x, y, offset_1, offset_2);
        }
    }
}