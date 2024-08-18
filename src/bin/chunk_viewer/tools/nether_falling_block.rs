use std::collections::{HashMap, HashSet};
use ggez::graphics::Color;
use ggegui::{egui, GuiContext};
use mc_utils::positions::{BlockPos, ChunkPos, Direction};
use tinyfiledialogs::MessageBoxIcon;
use mc_utils::cluster_finder12::HashClusterSet;
use mc_utils::flood_fill::{flood_fill, spider};
use mc_utils::world::{Dimension, World};
use std::sync::atomic::{AtomicUsize, Ordering};
use mc_utils::block_ids;
use mc_utils::litematica::{LitematicaBuilder, LitematicaRegionBuilder};
use std::fs::File;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::io::Write;
use crate::chunk_viewer::chunk_layer::{HashSetLayer, LayerGroup};
use crate::chunk_viewer::event_handler::{CommonState, State};
use crate::chunk_viewer::task_list::{Task, TaskList, TaskStatus};
use crate::chunk_viewer::tools::Tool;

pub struct NetherFallingBlockTool {
    world_path: Option<String>,
    fireless_chunks: HashSet<ChunkPos>,
    cluster: Vec<ChunkPos>,

    cluster_origin: ChunkPos,
    selecting_origin: bool,

    flood_fill_radius: u32,
    mask: i32,

    cluster_litematic_origin: Option<BlockPos>,
    fire_litematic_origin: Option<BlockPos>,
}

impl NetherFallingBlockTool {
    pub fn new() -> Self {
        Self {
            world_path: None,
            fireless_chunks: HashSet::new(),
            cluster: Vec::new(),
            cluster_origin: ChunkPos::new(0, 0),
            selecting_origin: false,
            flood_fill_radius: 100,
            mask: 4095,
            cluster_litematic_origin: None,
            fire_litematic_origin: None,
        }
    }
    fn update_flood_fill(&mut self, state: &mut CommonState) {
        self.cluster_litematic_origin = None;

        let flood_area = flood_fill(self.cluster_origin, &self.fireless_chunks, self.flood_fill_radius);

        let mut cluster_set = HashClusterSet::new(self.mask);
        for pos in flood_area.0.keys() {
            cluster_set.add_chunk(*pos);
        }

        let cluster = if let Some(interval) = cluster_set.largest_cluster() {
            interval.chunks.clone()
        } else {
            return;
        };
        self.cluster = cluster.clone();

        let spider_loader = spider(&cluster, &flood_area, |_, _| {});

        let nfb_layer = state.layers.get_layer_mut::<LayerGroup>("nether_falling_block").unwrap();

        let flood_layer = nfb_layer.get_layer_mut::<HashSetLayer>("flood_fill").unwrap();
        flood_layer.set_chunks(flood_area.0.keys().map(|c| *c).collect());
        let spider_layer = nfb_layer.get_layer_mut::<HashSetLayer>("spider").unwrap();
        spider_layer.set_chunks(spider_loader);
        let cluster_layer = nfb_layer.get_layer_mut::<HashSetLayer>("cluster").unwrap();
        cluster_layer.set_chunks(cluster.into_iter().collect());
        let cluster_origin_layer = nfb_layer.get_layer_mut::<HashSetLayer>("cluster_origin").unwrap();
        let mut cluster_origin_set = HashSet::new();
        cluster_origin_set.insert(self.cluster_origin);
        cluster_origin_layer.set_chunks(cluster_origin_set);
    }

    fn load_world(&mut self, world_path: String, task_list: &mut TaskList<State>) {
        let world_path2 = world_path.clone();
        let loading_task = Task::start_progress(move |tx| {
            let world = World::new(&world_path);

            let total_regions = world.get_num_regions(Dimension::Nether)?;
            let region_count = AtomicUsize::new(0);

            let result = world.region_pos_iter(Dimension::Nether)?.par_bridge().flat_map(|region_pos| -> anyhow::Result<Vec<ChunkPos>> {
                let mut region_fireless_chunks: Vec<ChunkPos> = Vec::new();
                let region = world.get_region_uncached(region_pos, Dimension::Nether)?.unwrap();
                for (chunk_pos, chunk) in region.chunk_iter() {
                    let mut has_fire = false;
                    for (_, block) in chunk.block_iter() {
                        if block.block_id == block_ids::FIRE {
                            has_fire = true;
                            break;
                        }
                    }
                    if !has_fire {
                        region_fireless_chunks.push(chunk_pos.offset(region_pos.into()))
                    }
                }

                let current_regions = region_count.fetch_add(1, Ordering::SeqCst) + 1;
                let progress = (current_regions as f32) / (total_regions as f32);
                tx.send(TaskStatus::Progress(progress))?;

                Ok(region_fireless_chunks)
            }).flatten().collect();
            Ok(result)
        }, move |result: anyhow::Result<Vec<ChunkPos>>, state: &mut State| {
            match result {
                Ok(result) => {
                    if let Some(tool) = state.toolbox.get_current_tool_mut::<NetherFallingBlockTool>() {
                        tool.world_path = Some(world_path2.clone());
                        tool.fireless_chunks = result.into_iter().collect();

                        let nfb_layer = state.common_state.layers.get_layer_mut::<LayerGroup>("nether_falling_block").unwrap();
                        let fireless_layer = nfb_layer.get_layer_mut::<HashSetLayer>("fireless").unwrap();
                        fireless_layer.set_chunks(tool.fireless_chunks.clone());

                        tool.update_flood_fill(&mut state.common_state);
                    } else {
                        println!("Tool switched before task finished!");
                    }
                }
                Err(err) => {
                    println!("Error finding fireless chunks {err}");
                    tinyfiledialogs::message_box_ok("Error", "Error finding fireless chunks, check console", MessageBoxIcon::Error);
                }
            }
        });
        task_list.add_task("Finding Fireless Chunks", loading_task);
    }

    fn generate_cluster_litematic(&mut self, task_list: &mut TaskList<State>, save_path: String) {
        let cluster = self.cluster.clone();
        let fireless_chunks = self.fireless_chunks.clone();
        let flood_fill_radius = self.flood_fill_radius;
        let cluster_origin = self.cluster_origin;
        let generate_task = Task::start(move || {
            let mut region = LitematicaRegionBuilder::new();

            for chunk in &cluster {
                region.set_block(BlockPos::new(8, 128, 8).offset((*chunk).into()), "hopper".into(), HashMap::new());
            }

            let flood_area = flood_fill(cluster_origin, &fireless_chunks, flood_fill_radius);
            spider(&cluster, &flood_area, |chunk, direction| {
                let block_offset = match direction {
                    Direction::North => BlockPos::new(8, 128, 15),
                    Direction::South => BlockPos::new(7, 128, 0),
                    Direction::East => BlockPos::new(0, 128, 8),
                    Direction::West => BlockPos::new(15, 128, 7),
                };
                region.set_block(block_offset.offset(chunk.into()), "chest".into(), HashMap::new());
            });

            let mut litematic = LitematicaBuilder::new();
            litematic.add_region("Cluster", region);
            litematic.save(&save_path, "Cluster")?;

            Ok(litematic.get_origin())
        }, |origin: anyhow::Result<BlockPos>, state: &mut State| {
            match origin {
                Ok(origin) => {
                    let tool = state.toolbox.get_current_tool_mut::<Self>().unwrap();
                    tool.cluster_litematic_origin = Some(origin);
                }
                Err(err) => {
                    println!("Error saving litematic {err}");
                    tinyfiledialogs::message_box_ok("Error", "Error generating litematic, check console", MessageBoxIcon::Error);
                }
            }
        });
        task_list.add_task("Cluster Litematic", generate_task);
    }

    fn generate_fire_litematic(&self, state: &CommonState, task_list: &mut TaskList<State>, save_path: String) {
        let selection = state.selection.clone();
        let world_path = self.world_path.as_ref().unwrap().clone();
        let generate_task = Task::start_progress(move |tx| {
            let mut world = World::new(&world_path);
            let mut litematic = LitematicaBuilder::new();
            for (i, chunk_pos) in selection.iter().enumerate() {
                if let Some(chunk) = world.get_chunk(*chunk_pos, Dimension::Nether)? {
                    for (block_pos, block) in chunk.block_iter() {
                        if block.block_id == block_ids::FIRE {
                            let mut region = LitematicaRegionBuilder::new();
                            region.set_block(block_pos, "sand".into(), HashMap::new());
                            litematic.add_region(&format!("fire.{}.{}.{}", block_pos.x, block_pos.y, block_pos.z), region);
                        }
                    }
                }
                let progress = (i as f32) / (selection.len() as f32);
                tx.send(TaskStatus::Progress(progress))?;
            }
            litematic.save(&save_path, "Fire Locations")?;
            Ok(BlockPos::new(0, 0, 0))
        }, |origin: anyhow::Result<BlockPos>, state: &mut State| {
            match origin {
                Ok(origin) => {
                    let tool = state.toolbox.get_current_tool_mut::<Self>().unwrap();
                    tool.fire_litematic_origin = Some(origin);
                }
                Err(err) => {
                    println!("Error saving litematic {err}");
                    tinyfiledialogs::message_box_ok("Error", "Error generating litematic, check console", MessageBoxIcon::Error);
                }
            }
        });
        task_list.add_task("Fire Litematic", generate_task);
    }

    fn export_chunks(&self, save_path: String) -> anyhow::Result<()> {
        let mut output_file = File::create(save_path)?;
        for chunk in &self.cluster {
            let line = format!("{},{}\n", chunk.x, chunk.z);
            output_file.write_all(line.as_bytes())?;
        }

        Ok(())
    }
}

impl Tool for NetherFallingBlockTool {
    fn start(&mut self, state: &mut CommonState) {
        let mut layer_group = LayerGroup::new();
        layer_group.add_layer("fireless", HashSetLayer::new(HashSet::new(), Color::from_rgb(255, 165, 0)), 1);
        layer_group.add_layer("flood_fill", HashSetLayer::new(HashSet::new(), Color::GREEN), 2);
        layer_group.add_layer("spider", HashSetLayer::new(HashSet::new(), Color::from_rgb(0, 128, 0)), 3);
        layer_group.add_layer("cluster", HashSetLayer::new(HashSet::new(), Color::BLUE), 4);
        layer_group.add_layer("cluster_origin", HashSetLayer::new(HashSet::new(), Color::MAGENTA), 5);
        state.layers.add_layer("nether_falling_block", layer_group, 0);
    }

    fn stop(&mut self, state: &mut CommonState) {
        state.layers.remove_layer("nether_falling_block");
    }

    fn gui(&mut self, state: &mut CommonState, task_list: &mut TaskList<State>, gui_ctx: &GuiContext) {
        egui::Window::new("Nether Falling Block").show(gui_ctx, |ui| {
            egui::Grid::new("nfb_cluster_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    if ui.button("Open World").clicked() {
                        let world_path = tinyfiledialogs::select_folder_dialog("Open Minecraft World", "");
                        if let Some(world_path) = world_path {
                            self.load_world(world_path, task_list);
                        }
                    }
                    ui.label(format!("Fireless Chunks: {}", self.fireless_chunks.len()));
                    ui.end_row();

                    if self.world_path.is_none() {
                        ui.set_enabled(false);
                    }

                    ui.label("Cluster Origin");
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut self.cluster_origin.x)).changed() {
                            self.update_flood_fill(state);
                        }
                        if ui.add(egui::DragValue::new(&mut self.cluster_origin.z)).changed() {
                            self.update_flood_fill(state);
                        }
                        if self.selecting_origin {
                            ui.add_enabled(false, egui::Button::new("⛶"));
                        } else {
                            if ui.button("⛶").clicked() {
                                self.selecting_origin = true;
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Flood Fill Radius");
                    if ui.add(egui::Slider::new(&mut self.flood_fill_radius, 0..=200)).changed() {
                        self.update_flood_fill(state);
                    }
                    ui.end_row();

                    ui.label("Hashmap Size");
                    egui::ComboBox::new("hashmap_size", "")
                        .selected_text(format!("{}", self.mask + 1))
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut self.mask, 2047, "2048").clicked() ||
                                ui.selectable_value(&mut self.mask, 4095, "4096").clicked() ||
                                ui.selectable_value(&mut self.mask, 8191, "8192").clicked() {
                                self.update_flood_fill(state);
                            }
                        });
                    ui.end_row();

                    ui.label(format!("Cluster Size: {}", self.cluster.len()));
                    let cluster_start = if self.cluster.len() > 0 {
                        self.cluster[0].hash(self.mask)
                    } else {
                        0
                    };
                    ui.label(format!("Cluster Start Hash: {}", cluster_start));
                    ui.end_row();

                    if ui.button("Cluster Litematic").clicked() {
                        let save_path = tinyfiledialogs::save_file_dialog_with_filter("Save Location", "", &["*.litematic"], ".litematic");
                        if let Some(save_path) = save_path {
                            self.generate_cluster_litematic(task_list, save_path);
                        }
                    }
                    if let Some(origin) = self.cluster_litematic_origin {
                        ui.label(format!("{:?}", origin));
                    }
                    ui.end_row();

                    if ui.button("Export Chunks").clicked() {
                        let save_path = tinyfiledialogs::save_file_dialog_with_filter("Export Location", "", &["*.csv"], ".csv");
                        if let Some(save_path) = save_path {
                            let result = self.export_chunks(save_path);
                            if let Err(error) = result {
                                println!("Error exporting chunks: {}", error);
                                tinyfiledialogs::message_box_ok("Error", "Error exporting chunks, check console", MessageBoxIcon::Error);
                            }
                        }
                    }
                    ui.end_row();

                    if ui.add_enabled(!state.selection.is_empty(), egui::Button::new("Fire Litematic")).clicked() {
                        let save_path = tinyfiledialogs::save_file_dialog_with_filter("Save Location", "", &["*.litematic"], ".litematic");
                        if let Some(save_path) = save_path {
                            self.generate_fire_litematic(state, task_list, save_path);
                        }
                    }
                    if let Some(origin) = self.fire_litematic_origin {
                        ui.label(format!("{:?}", origin));
                    }
                    ui.end_row();

                    if self.world_path.is_none() {
                        ui.set_enabled(true);
                    }
                });
        });
    }

    fn on_chunk_selected(&mut self, chunk: ChunkPos, state: &mut CommonState) {
        if self.selecting_origin {
            self.cluster_origin = chunk;
            self.selecting_origin = false;
            self.update_flood_fill(state);
        }
    }
}