use std::any::Any;
use std::collections::{HashMap, HashSet};
use ggez::Context;
use ggez::glam::vec2;
use ggez::graphics::{Canvas, Color, DrawParam, InstanceArray};
use mc_utils::positions::ChunkPos;
use crate::chunk_viewer::viewport::Viewport;

pub trait ChunkLayer: Any {
    fn render(&mut self, viewport: &Viewport, ctx: &Context, canvas: &mut Canvas);
}

struct LayerGroupEntry {
    layer: Box<dyn ChunkLayer>,
    z_index: i32,
    visible: bool
}
pub struct LayerGroup {
    layers: HashMap<String, LayerGroupEntry>,
    sorted_layers: Vec<String>,
}
impl LayerGroup {
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
            sorted_layers: Vec::new(),
        }
    }

    pub fn add_layer(&mut self, name: &str, layer: impl ChunkLayer, z_index: i32) {
        self.layers.insert(name.to_string(), LayerGroupEntry {
            layer: Box::new(layer),
            z_index,
             visible: true
        });
        self.sort_layers();
    }

    pub fn get_layer<C: ChunkLayer>(&mut self, name: &str) -> Option<&C> {
        let entry = self.layers.get(name)?;
        let layer = entry.layer.as_ref() as &dyn Any;
        Some(layer.downcast_ref::<C>()?)
    }

    pub fn get_layer_mut<C: ChunkLayer>(&mut self, name: &str) -> Option<&mut C> {
        let entry = self.layers.get_mut(name)?;
        let layer = entry.layer.as_mut() as &mut dyn Any;
        Some(layer.downcast_mut::<C>()?)
    }

    pub fn remove_layer(&mut self, name: &str) {
        self.layers.remove(name);
        self.sort_layers();
    }

    pub fn set_visible(&mut self, name: &str, visible: bool) {
        if let Some(entry) = self.layers.get_mut(name) {
            entry.visible = visible;
        }
    }

    fn sort_layers(&mut self) {
        let mut layers_entries: Vec<_> = self.layers.iter().collect();
        layers_entries.sort_by_key(|e| e.1.z_index);
        self.sorted_layers = layers_entries.iter().map(|e| e.0.clone()).collect();
    }
}
impl ChunkLayer for LayerGroup {
    fn render(&mut self, viewport: &Viewport, ctx: &Context, canvas: &mut Canvas) {
        for layer_name in &self.sorted_layers {
            let layer_entry = self.layers.get_mut(layer_name).unwrap();
            if layer_entry.visible {
                layer_entry.layer.render(viewport, ctx, canvas);
            }
        }
    }
}

pub struct ChunkColor {
    primary: Color,
    secondary: Option<Color>,
}
impl ChunkColor {
    pub fn new(primary: Color, secondary: Option<Color>) -> Self {
        Self {
            primary,
            secondary
        }
    }
}
impl Into<ChunkColor> for Color {
    fn into(self) -> ChunkColor {
        ChunkColor::new(self, None)
    }
}

pub trait VirtualChunkProvider {
    fn chunks_in_viewport(&self, viewport: &Viewport) -> impl Iterator<Item=(ChunkPos, impl Into<ChunkColor>)>;
    fn check_dirty(&mut self) -> bool;
}

pub struct VirtualChunkLayer<P: VirtualChunkProvider> {
    pub provider: P,
    instances: Option<InstanceArray>,
    rendered: HashSet<ChunkPos>,
    prev_viewport: Viewport,
}
impl<P: VirtualChunkProvider> VirtualChunkLayer<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            instances: None,
            rendered: HashSet::new(),
            prev_viewport: Viewport::new()
        }
    }

    fn update_instances(&mut self, viewport: &Viewport) {
        if let Some(instances) = &mut self.instances {
            if self.provider.check_dirty() {
                instances.clear();
                self.rendered.clear();
                self.prev_viewport = Viewport::new();
            }

            if self.prev_viewport != *viewport {
                for (chunk, color) in self.provider.chunks_in_viewport(viewport) {
                    if !self.rendered.contains(&chunk) {
                        self.rendered.insert(chunk);
                        let chunk_rect = viewport.chunk_to_rect(chunk);
                        let color = color.into();
                        instances.push(
                            DrawParam::new()
                                .dest(vec2(chunk_rect.x, chunk_rect.y))
                                .scale(vec2(chunk_rect.w, chunk_rect.h))
                                .color(color.primary)
                        );
                        if let Some(secondary) = color.secondary {
                            instances.push(
                                DrawParam::new()
                                    .dest(vec2(chunk_rect.x, chunk_rect.y))
                                    .scale(vec2(chunk_rect.w / 2.0, chunk_rect.h / 2.0))
                                    .color(secondary)
                            );
                        }
                    }
                }
                self.prev_viewport = viewport.clone();
            }
        }
    }
}
impl<P: VirtualChunkProvider + 'static> ChunkLayer for VirtualChunkLayer<P> {
    fn render(&mut self, viewport: &Viewport, ctx: &Context, canvas: &mut Canvas) {
        if self.instances.is_none() {
            self.instances = Some(InstanceArray::new(ctx, None));
        }
        self.update_instances(viewport);
        let instances = self.instances.as_ref().expect("Uninitialized InstanceArray??");
        canvas.draw(instances, [0.0, 0.0]);
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
    fn chunks_in_viewport(&self, viewport: &Viewport) -> impl Iterator<Item=(ChunkPos, impl Into<ChunkColor>)> {
        let tl = viewport.chunk_at(vec2(0.0, 0.0));
        let br = viewport.chunk_at(vec2(viewport.screen_width, viewport.screen_height));
        let mut chunks: HashSet<ChunkPos> = HashSet::new();
        if (tl.x..=br.x).contains(&tl.z) || (tl.z..=br.z).contains(&tl.x) {
            let mut x = tl.x;
            while x <= br.x && x <= br.z {
                chunks.insert(ChunkPos::new(x, x));
                x += 1;
            }
        }

        if (tl.x..=br.x).contains(&(-br.z)) || (tl.z..=br.z).contains(&(-tl.x)) {
            let mut x = tl.x;
            while x <= br.x && x <= -tl.z {
                chunks.insert(ChunkPos::new(x, -x));
                x += 1;
            }
        }

        chunks.into_iter().map(|c| (c, self.color))
    }

    fn check_dirty(&mut self) -> bool {
        false
    }
}

pub struct CheckerboardProvider<P: VirtualChunkProvider> {
    pub provider: P,
    highlight: Color,
    dirty: bool
}
impl<P: VirtualChunkProvider> CheckerboardProvider<P> {
    pub fn new(provider: P, highlight: Color) -> Self {
        Self {
            provider,
            highlight,
            dirty: false
        }
    }
}
impl<P: VirtualChunkProvider> VirtualChunkProvider for CheckerboardProvider<P> {
    fn chunks_in_viewport(&self, viewport: &Viewport) -> impl Iterator<Item=(ChunkPos, impl Into<ChunkColor>)> {
        self.provider.chunks_in_viewport(viewport).map(|(position, color)| {
            let mut color = color.into();
            let parity_x = position.x.abs() % 2;
            let parity_z = position.z.abs() % 2;
            if parity_x == parity_z {
                let (r, g, b, a) = color.primary.to_rgba();
                let (hr, hg, hb, ha) = self.highlight.to_rgba();
                let r = r.saturating_add(hr);
                let g = g.saturating_add(hg);
                let b = b.saturating_add(hb);
                let a = a.saturating_add(ha);
                color.primary = Color::from_rgba(r, g, b, a);
            }
            (position, color)
        })
    }

    fn check_dirty(&mut self) -> bool {
        let result = self.provider.check_dirty() || self.dirty;
        self.dirty = false;
        result
    }
}

pub struct HashSetLayer {
    color: Color,
    chunks: HashSet<ChunkPos>,
    dirty: bool,
    instances: Option<InstanceArray>
}
impl HashSetLayer {
    pub fn new(chunks: HashSet<ChunkPos>, color: Color) -> Self {
        Self {
            color,
            chunks,
            instances: None,
            dirty: false
        }
    }

    pub fn set_chunks(&mut self, chunks: HashSet<ChunkPos>) {
        self.chunks = chunks;
        self.dirty = true;
    }

    fn update_instances(&mut self, viewport: &Viewport) {
        if let Some(instances) = &mut self.instances {
            instances.clear();
            for chunk in &self.chunks {
                let chunk_rect = viewport.chunk_to_rect(*chunk);
                instances.push(
                    DrawParam::new()
                        .dest(vec2(chunk_rect.x, chunk_rect.y))
                        .scale(vec2(chunk_rect.w, chunk_rect.h))
                        .color(self.color)
                );
            }
        }
    }
}
impl ChunkLayer for HashSetLayer {
    fn render(&mut self, viewport: &Viewport, ctx: &Context, canvas: &mut Canvas) {
        if self.instances.is_none() {
            self.instances = Some(InstanceArray::new(ctx, None));
            self.update_instances(viewport);
        }
        if self.dirty {
            self.dirty = false;
            self.update_instances(viewport);
        }
        let instances = self.instances.as_ref().expect("InstanceArray is uninitialized?");
        canvas.draw(instances, [0.0, 0.0]);
    }
}