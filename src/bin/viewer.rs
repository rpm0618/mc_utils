#![feature(trait_upcasting)]
mod chunk_viewer;

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use ggegui::{egui, Gui};
use ggez::{Context, ContextBuilder, event, GameError, GameResult, graphics};
use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::MouseButton;
use ggez::glam::{vec2};
use ggez::graphics::{Color, DrawMode, DrawParam, Mesh};
use ggez::winit::dpi::LogicalSize;
use mc_utils::cluster_finder12::HashClusterSet;
use mc_utils::flood_fill::{flood_fill, spider};
use mc_utils::positions::{ChunkPos};
use crate::chunk_viewer::chunk_layer::{HashProvider, VirtualChunkLayer, DiagonalProvider, VecLayer, LayerMap};
use crate::chunk_viewer::viewport::Viewport;

struct MainState {
    viewport: Viewport,
    chunk_layers: LayerMap,

    gui: Gui,
    dragging: bool,

    flood_fill_radius: u32,

    selected_chunk: ChunkPos,

    fireless_chunks: HashSet<ChunkPos>,

    cluster: Vec<ChunkPos>,

    mask: i32
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let mut fireless_chunks: HashSet<ChunkPos> = HashSet::new();
        let mut fireless_file = File::open("out.csv")?;
        let mut fireless_str: String = String::new();
        fireless_file.read_to_string(&mut fireless_str)?;
        for line in fireless_str.lines() {
            let coords: Vec<&str> = line.split(",").collect();
            let x = coords[0].parse::<i32>().unwrap();
            let z = coords[1].parse::<i32>().unwrap();
            fireless_chunks.insert(ChunkPos::new(x, z));
        }

        let mut chunk_layers = LayerMap::new();
        // chunk_layers.add_layer("hashes", Box::new(VirtualChunkLayer::new(HashProvider::new(ChunkPos::new(0, 0), 4095), ctx)), 0);
        chunk_layers.add_layer("fireless", Box::new(VecLayer::from_csv("out.csv", Color::from_rgb(255, 165, 0), ctx)?), 1);
        chunk_layers.add_layer("flood_fill", Box::new(VecLayer::new(Vec::new(), Color::GREEN, ctx)), 2);
        chunk_layers.add_layer("spider", Box::new(VecLayer::new(Vec::new(), Color::from_rgb(0, 128, 0), ctx)), 3);
        chunk_layers.add_layer("cluster",Box::new(VecLayer::new(Vec::new(), Color::BLUE, ctx)), 4);
        chunk_layers.add_layer("diagonals", Box::new(VirtualChunkLayer::new(DiagonalProvider::new(Color::RED), ctx)), 10);

        Ok(MainState {
            viewport: Viewport::new(),
            chunk_layers,
            gui: Gui::new(ctx),
            dragging: false,
            selected_chunk: ChunkPos::new(0, 0),
            fireless_chunks,
            flood_fill_radius: 100,
            cluster: Vec::new(),
            mask: 4095
        })
    }

    fn update_instances(&mut self) {
        self.chunk_layers.update_viewport(&self.viewport);
    }

    fn update_flood_fill(&mut self) {
        let flood_area = flood_fill(self.selected_chunk, &self.fireless_chunks, self.flood_fill_radius);

        let mut cluster_set = HashClusterSet::new(self.mask);
        for pos in flood_area.0.keys() {
            cluster_set.add_chunk(*pos);
        }

        let mut index = 0;
        let mut largest_length = 0;

        for i in 0..cluster_set.intervals.len() {
            let interval = &cluster_set.intervals[i];
            if interval.chunks.len() > largest_length {
                index = i;
                largest_length = interval.chunks.len();
            }
        }

        let cluster = cluster_set.intervals[index].chunks.clone();

        self.cluster = cluster.clone();

        let spider_loader = spider(&cluster, &flood_area, |_, _| {});

        let flood_layer = self.chunk_layers.get_layer_mut::<VecLayer>("flood_fill").unwrap();
        flood_layer.set_chunks(flood_area.0.keys().map(|c| *c).collect());
        let spider_layer = self.chunk_layers.get_layer_mut::<VecLayer>("spider").unwrap();
        spider_layer.set_chunks(spider_loader.iter().map(|c| *c).collect());
        let cluster_layer = self.chunk_layers.get_layer_mut::<VecLayer>("cluster").unwrap();
        cluster_layer.set_chunks(cluster);
        self.update_instances();

        // let mut cluster_set = HashClusterSet::new(self.mask);
        //
        // for x in self.selected_chunk.x..(self.selected_chunk.x + self.flood_fill_radius as i32) {
        //     for z in self.selected_chunk.z..(self.selected_chunk.z + self.flood_fill_radius as i32) {
        //         let pos = ChunkPos::new(x, z);
        //         if self.fireless_chunks.contains(&pos) {
        //             cluster_set.add_chunk(pos);
        //         }
        //     }
        // }
        //
        // let mut index = 0;
        // let mut largest_length = 0;
        //
        // for i in 0..cluster_set.intervals.len() {
        //     let interval = &cluster_set.intervals[i];
        //     if interval.chunks.len() > largest_length {
        //         index = i;
        //         largest_length = interval.chunks.len();
        //     }
        // }
        //
        // let cluster = cluster_set.intervals[index].chunks.clone();
        // self.cluster = cluster.clone();
        // let cluster_layer = self.chunk_layers.get_layer_mut::<VecLayer>("cluster").unwrap();
        // cluster_layer.set_chunks(cluster);
        // self.update_instances();
    }
}

impl event::EventHandler<GameError> for MainState {
    fn update(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        let gui_ctx = self.gui.ctx();
        egui::Window::new("Info").movable(false).show(&gui_ctx, |ui| {
            if ui.button("Open World").clicked() {
                let world_path = tinyfiledialogs::select_folder_dialog("Open Minecraft World", "");
                if let Some(world_path) = world_path {
                    println!("{}", world_path);
                } else {
                    println!("No world selected");
                }
            }
            ui.label("Selected Chunk");
            ui.horizontal(|ui| {
                if ui.add(egui::DragValue::new(&mut self.selected_chunk.x)).changed() {
                    self.update_flood_fill();
                }
                if ui.add(egui::DragValue::new(&mut self.selected_chunk.z)).changed() {
                    self.update_flood_fill();
                }
            });

            ui.label("Flood Fill Radius");
            if ui.add(egui::DragValue::new(&mut self.flood_fill_radius)).changed() {
                self.update_flood_fill();
            }

            egui::ComboBox::from_label("Mask")
                .selected_text(format!("{}", self.mask))
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut self.mask, 2047, "2047").clicked() ||
                        ui.selectable_value(&mut self.mask, 4095, "4095").clicked() ||
                        ui.selectable_value(&mut self.mask, 8191, "8191").clicked() {
                        self.update_flood_fill();
                    }
                });

            ui.label(format!("Cluster Size: {}", self.cluster.len()))
        });
        self.gui.update(ctx);

        if ctx.mouse.button_just_released(MouseButton::Middle) {
            self.update_instances();
        }

        if ctx.mouse.button_just_released(MouseButton::Left) && !gui_ctx.is_pointer_over_area() {
            let chunk = self.viewport.chunk_at(ctx.mouse.position());
            self.selected_chunk = chunk;
            self.update_flood_fill();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::from([0.392, 0.584, 0.929, 1.0]));
        let original_screen_coords = canvas.screen_coordinates();
        let screen_coords = self.viewport.get_screen_coordinates();
        if original_screen_coords.is_some() {
            canvas.set_screen_coordinates(screen_coords);
        }

        for layer in self.chunk_layers.get_all_layers() {
            canvas.draw(layer.instances(), vec2(0.0, 0.0));
        }

        let chunk_rect = self.viewport.chunk_to_rect(self.viewport.chunk_at(ctx.mouse.position()));
        let chunk_highlight = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            chunk_rect,
            Color::from_rgba(0, 0, 0, 128)
        )?;
        canvas.draw(&chunk_highlight, vec2(0.0, 0.0));


        let selected_rect = self.viewport.chunk_to_rect(self.selected_chunk);
        let selected_chunk_highlight = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            selected_rect,
            Color::BLACK
        )?;
        canvas.draw(&selected_chunk_highlight, vec2(0.0, 0.0));

        if original_screen_coords.is_some() {
            canvas.set_screen_coordinates(original_screen_coords.unwrap());
        }
        canvas.draw(&self.gui, DrawParam::default());

        canvas.finish(ctx)?;

        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, _button: MouseButton, _x: f32, _y: f32) -> Result<(), GameError> {
        if !self.gui.ctx().is_pointer_over_area() {
            if _button == MouseButton::Middle {
                self.dragging = true
            }
        }
        Ok(())
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) -> Result<(), GameError> {
        if self.dragging && button == MouseButton::Middle {
            self.dragging = false;
        }
        Ok(())
    }


    fn mouse_motion_event(&mut self, _ctx: &mut Context, _x: f32, _y: f32, dx: f32, dy: f32) -> Result<(), GameError> {
        if self.dragging {
            self.viewport.translate(vec2(dx, dy));
        }
        Ok(())
    }

    fn mouse_wheel_event(&mut self, ctx: &mut Context, _x: f32, y: f32) -> Result<(), GameError> {
        if !self.gui.ctx().is_pointer_over_area() {
            self.viewport.zoom_into(y * 0.1, ctx.mouse.position());
            self.update_instances();
        }
        Ok(())
    }

    fn text_input_event(&mut self, _ctx: &mut Context, _character: char) -> Result<(), GameError> {
        self.gui.input.text_input_event(_character);
        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) -> Result<(), GameError> {
        self.viewport.on_resize(width, height, ctx.gfx.window().scale_factor() as f32);
        self.gui.input.set_scale_factor(self.viewport.scale_factor * self.viewport.zoom, (width, height));
        self.update_instances();
        Ok(())
    }


}

fn main() -> GameResult {
    let (mut ctx, event_loop) = ContextBuilder::new("viewer", "rpm0618")
        .window_setup(WindowSetup {
            title: "Spider Cluster Finder".to_string(),
            ..WindowSetup::default()
        })
        .window_mode(WindowMode {
            logical_size: Some(LogicalSize::new(800.0, 600.0)),
            resizable: true,
            ..WindowMode::default()
        })
        .build()?;
    let state = MainState::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
}