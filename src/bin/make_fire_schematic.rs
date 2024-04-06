use std::collections::HashMap;
use std::io::Error;
use ggegui::egui::Key::P;
use quartz_nbt::{NbtCompound, NbtList};
use mc_utils::positions::{BlockPos, ChunkPos};
use mc_utils::litematica::{LitematicaBuilder, LitematicaRegionBuilder};
use mc_utils::world::{Dimension, World};

fn add_fire_to_litematic(world: &mut World, chunk_pos: ChunkPos, litematic: &mut LitematicaRegionBuilder) -> Result<i32, Error> {
    let chunk = world.get_chunk(chunk_pos, Dimension::Nether)?;

    if chunk.is_none() {
        println!("Missing Chunk: {:?}", chunk_pos);
        return Ok(0);
    }

    let chunk = chunk.unwrap();

    let level_data: &NbtCompound = chunk.data.get("Level").unwrap();
    let sections: &NbtList = level_data.get("Sections").unwrap();

    let mut total_fire = 0;

    for i in 0..sections.len() {
        let section: &NbtCompound = sections.get(i).unwrap();
        let subchunk: u8 = section.get("Y").unwrap();
        let blocks: &[u8] = section.get("Blocks").unwrap();
        for index in 0..4096 {
            let block_id = blocks[index];
            if block_id == 51 {
                let x_offset = index & 0xf;
                let y_offset = (index >> 8) & 0xf;
                let z_offset = (index >> 4) & 0xf;

                let block_x = chunk_pos.x << 4 | x_offset as i32;
                let block_y = (subchunk as i32) << 4 | y_offset as i32;
                let block_z = chunk_pos.z << 4 | z_offset as i32;

                total_fire += 1;

                litematic.set_block((block_x, block_y, block_z).into(), "sand".into(), HashMap::new());
            }
        }
    }

    Ok(total_fire)
}
fn main() -> Result<(), Error> {
    // let mut world = World::new("C:\\Ryan\\Personal\\minecraft\\carpetmod112\\server\\proto-test");
    let mut world = World::new("C:\\Ryan\\Personal\\minecraft\\prototech\\falling_block\\83862b5c-2ba6-481c-9b20-6cfa861babfe\\world");

    //
    // let mut region1 = LitematicaRegionBuilder::new();
    // let mut region2 = LitematicaRegionBuilder::new();
    //
    // let mut total_fire = 0;
    //
    // for z in -188..=-6 {
    //     let x = 0;
    //     total_fire += add_fire_to_litematic(&mut world, ChunkPos::new(x, z), &mut region1)?;
    // }
    //
    // for x in -126..=1 {
    //     let z = -188;
    //     total_fire += add_fire_to_litematic(&mut world, ChunkPos::new(x, z), &mut region2)?;
    // }
    //
    // println!("Total Fires: {}", total_fire);
    //
    // let mut litematic = LitematicaBuilder::new();
    // litematic.add_region("region1", region1);
    // litematic.add_region("region2", region2);
    //
    // litematic.save("C:\\Ryan\\Personal\\minecraft\\MultiMC\\instances\\1.12.22\\.minecraft\\schematics\\mobswitch_fires.litematic", "Mobswitch Fires");
    // println!("Origin {:?}", litematic.get_origin());

    // let lines = [
    //     (ChunkPos::new(17, -3), ChunkPos::new(52, -3)),
    //     (ChunkPos::new(52, -2), ChunkPos::new(52, 8)),
    //     (ChunkPos::new(53, 8), ChunkPos::new(75, 8)),
    //     (ChunkPos::new(75, 9), ChunkPos::new(75, 73)),
    // ];
    // 
    // let mut total_fire = 0;
    // 
    // let mut litematic = LitematicaBuilder::new();
    // 
    // for i in 0..lines.len() {
    //     let line = lines[i];
    //     let mut region = LitematicaRegionBuilder::new();
    // 
    //     let mut fire_in_line = 0;
    // 
    //     if line.0.x == line.1.x {
    //         for z in i32::min(line.0.z, line.1.z)..=i32::max(line.0.z, line.1.z) {
    //             fire_in_line += add_fire_to_litematic(&mut world, ChunkPos::new(line.0.x, z), &mut region)?;
    //         }
    //     } else {
    //         for x in i32::min(line.0.x, line.1.x)..=i32::max(line.0.x, line.1.x) {
    //             fire_in_line += add_fire_to_litematic(&mut world, ChunkPos::new(x, line.0.z), &mut region)?;
    //         }
    //     }
    // 
    //     if fire_in_line > 0 {
    //         total_fire += fire_in_line;
    //         litematic.add_region(&format!("line{}", i), region);
    //     }
    // }
    // 
    // let diagonals = [
    //     (ChunkPos::new(75, 73), ChunkPos::new(276, 273)),
    //     (ChunkPos::new(75, 74), ChunkPos::new(276, 274)),
    //     (ChunkPos::new(75, 75), ChunkPos::new(276, 275)),
    // ];
    // 
    // for i in 0..diagonals.len() {
    //     let diag = diagonals[i];
    //     for step in 0..=(diag.1.x - diag.0.x) {
    //         let mut region = LitematicaRegionBuilder::new();
    //         let fire_in_chunk = add_fire_to_litematic(&mut world, ChunkPos::new(diag.0.x + step, diag.0.z + step), &mut region)?;
    //         if fire_in_chunk > 0 {
    //             litematic.add_region(&format!("diag{}.{}", i, step), region);
    //             total_fire += fire_in_chunk;
    //         }
    //     }
    // }

    let mut total_fire = 0;
    let mut litematic = LitematicaBuilder::new();

    let mut region = LitematicaRegionBuilder::new();

    for x in -302..=-269 {
        for z in 619..=653 {
            total_fire += add_fire_to_litematic(&mut world, (x, z).into(), &mut region)?;
        }
    }

    litematic.add_region("region", region);

    println!("Total Fires: {}", total_fire);

    litematic.save("C:\\Ryan\\Personal\\minecraft\\MultiMC\\instances\\1.12.22\\.minecraft\\schematics\\setup_fires.litematic", "Setup Fires");
    println!("Origin {:?}", litematic.get_origin());

    Ok(())
}