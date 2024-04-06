use std::any::Any;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use ggez::{Context, GameResult};
use ggez::glam::vec2;
use ggez::graphics::{Color, DrawParam, InstanceArray};
use mc_utils::positions::{ChunkPos, HashV1_12};
use crate::chunk_viewer::viewport::Viewport;

pub trait ChunkLayer: Any {
    fn instances(&self) -> &InstanceArray;
    fn update_viewport(&mut self, viewport: &Viewport);
}

struct Layer {
    layer: Box<dyn ChunkLayer>,
    z_index: i32
}
pub struct LayerMap {
    layer_map: HashMap<String, Layer>
}
impl LayerMap {
    pub fn new() -> Self {
        Self {
            layer_map: HashMap::new()
        }
    }

    pub fn add_layer(&mut self, layer_name: &str, layer: Box<dyn ChunkLayer>, z_index: i32) {
        self.layer_map.insert(layer_name.to_string(), Layer {
            layer,
            z_index
        });
    }

    pub fn get_layer<C>(&self, layer_name: &String) -> Option<&C>
        where C: ChunkLayer
    {
        if let Some(layer_box) = self.layer_map.get(layer_name) {
            let layer_any = layer_box.layer.as_ref() as &dyn Any;
            if let Some(layer) = layer_any.downcast_ref::<C>() {
                return Some(layer)
            }
        }
        None
    }

    pub fn get_layer_mut<C>(&mut self, layer_name: &str) -> Option<&mut C>
        where C: ChunkLayer
    {
        if let Some(layer_box) = self.layer_map.get_mut(layer_name) {
            let layer_any = layer_box.layer.as_mut() as &mut dyn Any;
            if let Some(layer) = layer_any.downcast_mut::<C>() {
                return Some(layer)
            }
        }
        None
    }

    pub fn get_all_layers(&self) -> Vec<&Box<dyn ChunkLayer>> {
        let mut layers: Vec<_> = self.layer_map.values().collect();
        layers.sort_by_key(|l| l.z_index);
        layers.iter().map(|l| &l.layer).collect()
    }

    pub fn update_viewport(&mut self, viewport: &Viewport) {
        let layers: Vec<_> = self.layer_map.values_mut().collect();
        for layer in layers {
            layer.layer.update_viewport(viewport)
        }
    }
}

pub trait VirtualChunkProvider {
    fn color_for_chunk(&self, chunk: ChunkPos) -> Option<Color>;
}

pub struct VirtualChunkLayer<P: VirtualChunkProvider> {
    chunk_instances: InstanceArray,
    chunk_provider: P
}
impl<P> VirtualChunkLayer<P> where P: VirtualChunkProvider {
    pub fn new(chunk_provider: P, ctx: &Context) -> Self {
        Self {
            chunk_provider,
            chunk_instances: InstanceArray::new(ctx, None),
        }
    }
}
impl<P> ChunkLayer for VirtualChunkLayer<P> where P: VirtualChunkProvider + 'static {
    fn instances(&self) -> &InstanceArray {
        &self.chunk_instances
    }

    fn update_viewport(&mut self, viewport: &Viewport) {
        self.chunk_instances.clear();

        let top_left_chunk = viewport.chunk_at(vec2(0.0, 0.0));
        let bottom_right_chunk = viewport.chunk_at(vec2(viewport.screen_width, viewport.screen_height));

        for x in (top_left_chunk.x)..=(bottom_right_chunk.x) {
            for z in (top_left_chunk.z)..=(bottom_right_chunk.z) {
                let chunk = ChunkPos::new(x, z);
                if let Some(color) = self.chunk_provider.color_for_chunk(chunk) {
                    let chunk_rect = viewport.chunk_to_rect(chunk);
                    self.chunk_instances.push(
                        DrawParam::new()
                            .dest(vec2(chunk_rect.x, chunk_rect.y))
                            .scale(vec2(chunk_rect.w, chunk_rect.h))
                            .color(color)
                    )
                }
            }
        }
    }
}

fn ease_out_expo(x: f32) -> f32 {
    if x == 1.0 {
        1.0
    } else {
        1.0 - f32::powf(2.0, -10.0 * x)
    }
}

pub struct HashProvider {
    target_chunk: ChunkPos,
    mask: i32
}
impl HashProvider {
    pub fn new(target_chunk: ChunkPos, mask: i32) -> Self {
        Self {
            target_chunk, mask
        }
    }
}
impl VirtualChunkProvider for HashProvider {
    fn color_for_chunk(&self, chunk: ChunkPos) -> Option<Color> {
        let hash = chunk.hash::<HashV1_12>(self.mask);
        let target_hash = self.target_chunk.hash::<HashV1_12>(self.mask);
        let diff = (target_hash - hash).abs();
        let ratio = ease_out_expo((diff as f32) / (self.mask as f32));

        Some(Color::from([1.0, ratio, ratio, 1.0]))
    }
}

pub struct DiagonalProvider {
    color: Color
}
impl DiagonalProvider {
    pub fn new(color: Color) -> Self {
        Self {
            color
        }
    }
}
impl VirtualChunkProvider for DiagonalProvider {
    fn color_for_chunk(&self, chunk: ChunkPos) -> Option<Color> {
        if chunk.x == chunk.z || chunk.x == -chunk.z {
            Some(self.color)
        } else {
            None
        }
    }
}

pub struct VecLayer {
    chunk_instances: InstanceArray,
    chunks: Vec<ChunkPos>,
    color: Color,
    prev_chunk_size: f32,
    dirty: bool
}
impl VecLayer {
    pub fn new(chunks: Vec<ChunkPos>, color: Color, ctx: &Context) -> Self {
        let chunk_instances = InstanceArray::new(ctx, None);
        Self {
            chunk_instances,
            chunks,
            color,
            prev_chunk_size: 0.0,
            dirty: false
        }
    }

    pub fn from_csv<P: AsRef<Path>>(path: P, color: Color, ctx: &Context) -> GameResult<Self> {
        let mut chunks: Vec<ChunkPos> = Vec::new();
        let chunks_file = File::open(path)?;
        for line in BufReader::new(chunks_file).lines() {
            let line = line?;
            let coords: Vec<&str> = line.split(",").collect();
            let x = coords[0].parse::<i32>().unwrap();
            let z = coords[1].parse::<i32>().unwrap();
            chunks.push(ChunkPos::new(x, z));
        }


        let chunk_instances = InstanceArray::new(ctx, None);
        Ok(Self {
            chunk_instances,
            chunks,
            color,
            prev_chunk_size: 0.0,
            dirty: false
        })
    }

    pub fn set_chunks(&mut self, chunks: Vec<ChunkPos>) {
        self.chunks = chunks;
        self.dirty = true;
    }
}
impl ChunkLayer for VecLayer {
    fn instances(&self) -> &InstanceArray {
        &self.chunk_instances
    }

    fn update_viewport(&mut self, viewport: &Viewport) {
        if self.prev_chunk_size != viewport.chunk_size || self.dirty {
            self.prev_chunk_size = viewport.chunk_size;
            self.dirty = false;
            self.chunk_instances.clear();
            for chunk in &self.chunks {
                let chunk_rect = viewport.chunk_to_rect(*chunk);
                self.chunk_instances.push(
                    DrawParam::new()
                        .dest(vec2(chunk_rect.x, chunk_rect.y))
                        .scale(vec2(chunk_rect.w, chunk_rect.h))
                        .color(self.color)
                );
            }
        }
    }
}