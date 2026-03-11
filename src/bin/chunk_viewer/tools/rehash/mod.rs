use std::collections::HashSet;
use ggez::graphics::Color;
use mc_utils::chm_simulator;
use mc_utils::positions::ChunkPos;
use crate::chunk_viewer::chunk_layer::{ChunkColor, VirtualChunkProvider};
use crate::chunk_viewer::viewport::Viewport;

pub mod tool;

pub struct ValidStallChunkProvider {
    anvil_mask: i32,
    rehash_chunk: ChunkPos,
    enabled: bool,
    dirty: bool
}
impl ValidStallChunkProvider {
    pub fn new() -> Self {
        Self {
            anvil_mask: 15,
            rehash_chunk: ChunkPos::new(0, 0),
            enabled: false,
            dirty: true
        }
    }

    pub fn set_anvil_mask(&mut self, mask: i32) {
        if mask == self.anvil_mask {
            return;
        }
        self.anvil_mask = mask;
        self.dirty = true;
    }

    pub fn set_rehash_chunk(&mut self, rehash_chunk: ChunkPos) {
        if rehash_chunk == self.rehash_chunk {
            return;
        }
        self.rehash_chunk = rehash_chunk;
        self.dirty = true;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled == self.enabled {
            return;
        }
        self.enabled = enabled;
        self.dirty = true;
    }

    fn is_valid_stall_chunk(&self, chunk: &ChunkPos) -> bool {
        let rehash_chunk_bin = chm_simulator::bin_for(self.rehash_chunk, self.anvil_mask);
        let chunk_bin = chm_simulator::bin_for(*chunk, self.anvil_mask);
        chunk_bin < rehash_chunk_bin
    }
}
impl VirtualChunkProvider for ValidStallChunkProvider {
    fn chunks_in_viewport(&self, viewport: &Viewport, already_rendered: &HashSet<ChunkPos>) -> Vec<(ChunkPos, ChunkColor)> {
        if !self.enabled {
            return vec![];
        }
        viewport.chunk_iter()
            .filter(|c| !already_rendered.contains(c))
            .filter(|c| self.is_valid_stall_chunk(c))
            .map(|c| (c, ChunkColor::new(Color::from_rgba(0, 0, 0, 0), Some(Color::WHITE))))
            .collect()
    }

    fn check_dirty(&mut self) -> bool {
        let result = self.dirty;
        self.dirty = false;
        result
    }
}

pub struct ValidGlassChunkProvider {
    mask: i32,
    target_cluster_size: usize,
    tolerance: usize,
    enabled: bool,
    dirty: bool
}
impl ValidGlassChunkProvider {
    pub fn new(mask: i32, target_cluster_size: usize, tolerance: usize) -> Self {
        Self {
            mask,
            target_cluster_size,
            tolerance,
            enabled: false,
            dirty: true
        }
    }

    pub fn set_mask(&mut self, mask: i32) {
        if mask == self.mask {
            return;
        }
        self.mask = mask;
        self.dirty = true;
    }

    pub fn set_target_cluster_size(&mut self, target_cluster_size: usize) {
        if target_cluster_size == self.target_cluster_size {
            return;
        }
        self.target_cluster_size = target_cluster_size;
        self.dirty = true;
    }

    pub fn set_tolerance(&mut self, tolerance: usize) {
        if tolerance == self.tolerance {
            return;
        }
        self.tolerance = tolerance;
        self.dirty = true;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if enabled == self.enabled {
            return;
        }
        self.enabled = enabled;
        self.dirty = true;
    }

    fn is_valid_chunk(&self, chunk: &ChunkPos) -> bool {
        let max_hash = self.mask.saturating_sub(chunk.hash(self.target_cluster_size as i32)) + self.tolerance as i32;
        let min_hash = self.mask.saturating_sub(chunk.hash(self.target_cluster_size as i32)).saturating_sub(self.tolerance as i32);

        let chunk_hash = chunk.hash(self.mask);
        if chunk_hash >= min_hash && chunk_hash <= max_hash {
            let next_mask = ((self.mask + 1) * 2) - 1;
            let next_hash = chunk.hash(next_mask);
            return next_hash == chunk_hash;
        }

        false
    }
}
impl VirtualChunkProvider for ValidGlassChunkProvider {
    fn chunks_in_viewport(&self, viewport: &Viewport, already_rendered: &HashSet<ChunkPos>) -> Vec<(ChunkPos, ChunkColor)> {
        if !self.enabled {
            return vec![];
        }

        viewport.chunk_iter()
            .filter(|c| !already_rendered.contains(c))
            .filter(|c| self.is_valid_chunk(c))
            .map(|c| (c, ChunkColor::new(Color::from_rgba(0, 0, 0, 0), Some(Color::YELLOW))))
            .collect()
    }

    fn check_dirty(&mut self) -> bool {
        let result = self.dirty;
        self.dirty = false;
        result
    }
}