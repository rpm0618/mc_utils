use std::collections::{BinaryHeap, HashMap};
use bitflags::bitflags;
use indexmap::IndexSet;
use java_string::JavaString;
use valence_nbt::{compound, List};
use crate::block::Block;
use crate::block_ids::{CHEST, HOPPER};
use crate::error::Result;
use crate::kdtree_chunkpos::ChunkPosKdTree;
use crate::litematica::{LitematicaBuilder, LitematicaRegionBuilder};
use crate::positions::{BlockPos, ChunkPos};
use crate::world::{Dimension, World};

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    pub struct Directions: u8 {
        const North = 1 << 0;
        const South = 1 << 1;
        const East = 1 << 2;
        const West = 1 << 3;
        const Load = 1 << 4;
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
struct RsaEdge {
    chunk: ChunkPos,
    parent: ChunkPos,
    dist: u32,
}

pub type Arborescence = HashMap<ChunkPos, Directions>;

impl RsaEdge {
    fn new(chunk: ChunkPos, parent: ChunkPos) -> Self {
        RsaEdge { chunk, parent, dist: i32::abs_diff(chunk.x, parent.x) + i32::abs_diff(chunk.z, parent.z) }
    }
}
impl Ord for RsaEdge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.dist.cmp(&self.dist)
    }
}
impl PartialOrd for RsaEdge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Computes an approximate Rectilinear Spanning Arborescence (RSA) of the given chunks, rooted at
// the given origin. Uses Prim's algorithm and a kd-tree to speed up neighbor finding.
// The result is a map from each chunk to the set of directions that correspond to edges directed
// away from the chunk (including all chunks needed to load the target set).
// The result can be passed to the render_arborescence_* methods
pub fn compute_arborescence(chunks: &IndexSet<ChunkPos>, origin: ChunkPos) -> Arborescence {
    let mut all_chunks: Vec<ChunkPos> = chunks.iter().cloned().collect();
    all_chunks.push(origin);

    let mut result = Arborescence::with_capacity(all_chunks.len());

    for chunk in chunks {
        result.insert(*chunk, Directions::Load);
    }

    let mut kdtree = ChunkPosKdTree::build(all_chunks);
    let mut queue = BinaryHeap::new();

    let mut in_tree = IndexSet::with_capacity(chunks.len() + 1);
    in_tree.insert(origin);

    for chunk in kdtree.nearest(&origin, 20) {
        queue.push(RsaEdge::new(chunk, origin))
    }

    while let Some(edge) = queue.pop() {
        let chunk = edge.chunk;
        if in_tree.contains(&chunk) {
            continue;
        }
        in_tree.insert(chunk);
        kdtree.remove(&chunk);

        build_edge(&edge, &mut result);

        for neighbor in kdtree.nearest(&chunk, 20) {
            if !in_tree.contains(&neighbor) {
                queue.push(RsaEdge::new(neighbor, chunk));
            }
        }
    }

    result
}

// Build the edge from a given chunk to its parent, attempting to maximize overlap with existing
// built edges.
fn build_edge(edge: &RsaEdge, arbor: &mut Arborescence) {
    let x_min = edge.chunk.x.min(edge.parent.x);
    let x_max = edge.chunk.x.max(edge.parent.x);
    let z_min = edge.chunk.z.min(edge.parent.z);
    let z_max = edge.chunk.z.max(edge.parent.z);

    let x_path = {
        let mut path = (x_min..=x_max).map(|x| ChunkPos::new(x, edge.chunk.z)).collect::<Vec<_>>();
        path.extend((z_min..=z_max).map(|z| ChunkPos::new(edge.parent.x, z)).collect::<Vec<_>>());
        path
    };
    let z_path = {
        let mut path = (z_min..=z_max).map(|z| ChunkPos::new(edge.chunk.x, z)).collect::<Vec<_>>();
        path.extend((x_min..=x_max).map(|x| ChunkPos::new(x, edge.parent.z)).collect::<Vec<_>>());
        path
    };

    let x_overlap = x_path.iter().filter(|c| arbor.contains_key(*c)).count();
    let z_overlap = z_path.iter().filter(|c| arbor.contains_key(*c)).count();

    if x_overlap > z_overlap {
        build_edge_x(edge, arbor, edge.parent.z);
        build_edge_z(edge, arbor, edge.chunk.x);
    } else {
        build_edge_z(edge, arbor, edge.parent.x);
        build_edge_x(edge, arbor, edge.chunk.z);
    }
}

fn build_edge_x(edge: &RsaEdge, arbor: &mut Arborescence, z: i32) {
    let start_x = edge.parent.x;
    let end_x = edge.chunk.x;

    if start_x <= end_x {
        let path = (start_x..end_x).map(|x| ChunkPos::new(x, z)).collect::<Vec<_>>();
        for chunk in path {
            arbor.insert(chunk, *arbor.get(&chunk).unwrap_or(&Directions::empty()) | Directions::East);
        }
    } else {
        let path = (end_x+1..=start_x).map(|x| ChunkPos::new(x, z)).collect::<Vec<_>>();
        for chunk in path {
            arbor.insert(chunk, *arbor.get(&chunk).unwrap_or(&Directions::empty()) | Directions::West);
        }
    }
}

fn build_edge_z(edge: &RsaEdge, arbor: &mut Arborescence, x: i32) {
    let start_z = edge.parent.z;
    let end_z = edge.chunk.z;
    if start_z <= end_z {
        let path = (start_z..end_z).map(|z| ChunkPos::new(x, z)).collect::<Vec<_>>();
        for chunk in path {
            arbor.insert(chunk, *arbor.get(&chunk).unwrap_or(&Directions::empty()) | Directions::South);
        }
    } else {
        let path = (end_z+1..=start_z).map(|z| ChunkPos::new(x, z)).collect::<Vec<_>>();
        for chunk in path {
            arbor.insert(chunk, *arbor.get(&chunk).unwrap_or(&Directions::empty()) | Directions::North);
        }
    }
}

// Return a set of all chunks in the given arborescence.
pub fn render_arborescence_set(arbor: &Arborescence) -> IndexSet<ChunkPos> {
    let mut mst = IndexSet::new();
    for chunk in arbor.keys() {
        mst.insert(*chunk);
    }
    mst
}

// Generate a litematic from the given arborescence. Chests are used to load the chunks, and if
// permaload is true, a hopper is placed in the cluster chunks to keep them loaded
pub fn render_arborescence_litematic(arbor: &Arborescence, litematica: &mut LitematicaBuilder, permaload: bool) {
    let mut region = LitematicaRegionBuilder::new();

    let hopper_pos = BlockPos::new(8, 251, 8);
    let north_pos = BlockPos::new(8, 251, 0);
    let south_pos = BlockPos::new(8, 251, 15);
    let east_pos = BlockPos::new(15, 251, 8);
    let west_pos = BlockPos::new(0, 251, 8);

    for (chunk, directions) in arbor {
        if permaload && directions.contains(Directions::Load) {
            region.set_block(hopper_pos.offset((*chunk).into()), "hopper".into(), HashMap::new());
        }
        if directions.contains(Directions::North) {
            region.set_block(north_pos.offset((*chunk).into()), "chest".into(), HashMap::new());
        }
        if directions.contains(Directions::South) {
            region.set_block(south_pos.offset((*chunk).into()), "chest".into(), HashMap::new());
        }
        if directions.contains(Directions::East) {
            region.set_block(east_pos.offset((*chunk).into()), "chest".into(), HashMap::new());
        }
        if directions.contains(Directions::West) {
            region.set_block(west_pos.offset((*chunk).into()), "chest".into(), HashMap::new());
        }
    }

    litematica.add_region("cluster", region);
}

// Build the arborescence into a Minecraft world. Chests are used to load the chunks, and if
// permaload is true, a hopper is placed in the cluster chunks to keep them loaded.
// All the chunks in the arborescence must already exist.
pub fn render_arborescence_world(arbor: &Arborescence, world: &World, permaload: bool) -> Result<()> {
    let hopper_pos = BlockPos::new(8, 251, 8);
    let north_pos = BlockPos::new(8, 251, 0);
    let south_pos = BlockPos::new(8, 251, 15);
    let east_pos = BlockPos::new(15, 251, 8);
    let west_pos = BlockPos::new(0, 251, 8);

    let hopper = Block::new(HOPPER, 0);
    let hopper_te = compound! { <JavaString>
        "id" => "minecraft:hopper",
        "Items" => List::Compound(vec![]),
        "Lock" => "",
        "TransferCooldown" => 0
    };

    let chest = Block::new(CHEST, 2);
    let chest_te = compound! { <JavaString>
        "id" => "minecraft:chest",
        "Items" => List::Compound(vec![]),
        "Lock" => ""
    };

    for (chunk, directions) in arbor {
        if permaload && directions.contains(Directions::Load) {
            world.set_block_with_te(hopper_pos.offset((*chunk).into()), Dimension::Overworld, hopper.clone(), hopper_te.clone())?;
        }
        if directions.contains(Directions::North) {
            world.set_block_with_te(north_pos.offset((*chunk).into()), Dimension::Overworld, chest.clone(), chest_te.clone())?;
        }
        if directions.contains(Directions::South) {
            world.set_block_with_te(south_pos.offset((*chunk).into()), Dimension::Overworld, chest.clone(), chest_te.clone())?;
        }
        if directions.contains(Directions::East) {
            world.set_block_with_te(east_pos.offset((*chunk).into()), Dimension::Overworld, chest.clone(), chest_te.clone())?;
        }
        if directions.contains(Directions::West) {
            world.set_block_with_te(west_pos.offset((*chunk).into()), Dimension::Overworld, chest.clone(), chest_te.clone())?;
        }
    }

    Ok(())
}