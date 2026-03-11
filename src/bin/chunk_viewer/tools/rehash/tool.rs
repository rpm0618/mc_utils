use std::cmp::{max, min};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::{bail, Context};
use egui::{RichText, Color32};
use ggez::graphics::Color;
use indexmap::IndexSet;
use rayon::prelude::*;
use tinyfiledialogs::{message_box_ok, MessageBoxIcon, YesNo};
use mc_utils::chm_simulator::bin_for;
use mc_utils::litematica::LitematicaBuilder;
use mc_utils::mst::{compute_arborescence, render_arborescence_litematic, render_arborescence_set, render_arborescence_world, Arborescence};
use mc_utils::positions::{BlockPos, ChunkPos};
use mc_utils::rehash_cluster_finder12::RehashClusterFinder;
use mc_utils::world::{Dimension, World};
use crate::chunk_viewer::chunk_layer::{HashSetLayer, LayerGroup, VirtualChunkLayer};
use crate::chunk_viewer::event_handler::{CommonState, State};
use crate::chunk_viewer::gui::GuiContext;
use crate::chunk_viewer::task_list::{Task, TaskList, TaskStatus};
use crate::chunk_viewer::tools::rehash::{ValidGlassChunkProvider, ValidStallChunkProvider};
use crate::chunk_viewer::tools::Tool;

enum SelectionMode {
    GlassChunk,
    ViewOrigin,
    RehashChunk,
}

struct GlassChunkSubtool {
    enabled: bool,
    tolerance: usize,
}
impl GlassChunkSubtool {
    fn new() -> Self {
        Self {
            enabled: false,
            tolerance: 100,
        }
    }
}

struct ClusterSubtool {
    possible_cluster_chunks: IndexSet<ChunkPos>,
    cluster_chunks: IndexSet<ChunkPos>,
    mst_chunks: IndexSet<ChunkPos>,
    arborescence: Arborescence
}
impl ClusterSubtool {
    fn new() -> Self {
        Self {
            possible_cluster_chunks: IndexSet::new(),
            cluster_chunks: IndexSet::new(),
            mst_chunks: IndexSet::new(),
            arborescence: Arborescence::new(),
        }
    }

    fn invalidate_cluster(&mut self) {
        self.cluster_chunks.clear();
        self.mst_chunks.clear();
    }
}

struct StallChunkSubtool {
    enabled: bool,
    anvil_mask: i32,
    stall_chunks: HashSet<ChunkPos>,
}
impl StallChunkSubtool {
    fn new() -> Self {
        Self {
            enabled: false,
            anvil_mask: 15,
            stall_chunks: HashSet::new(),
        }
    }
}

pub struct RehashTool {
    world_name: Option<String>,
    world_path: Option<String>,
    spawn_point: Option<BlockPos>,
    spawn_chunks: HashSet<ChunkPos>,

    possible_rehash_chunks: HashMap<Dimension, HashSet<ChunkPos>>,
    non_rehash_chunks: HashMap<Dimension, HashSet<ChunkPos>>,

    permanent_extra_chunks: HashSet<ChunkPos>,
    temp_extra_chunks: HashSet<ChunkPos>,

    mask: i32,
    target_cluster_size: usize,
    dimension: Dimension,

    view_origin: ChunkPos,
    view_distance: i32,

    glass_chunk: ChunkPos,
    rehash_chunk: ChunkPos,

    selection_mode: Option<SelectionMode>,

    loaded_chunks: HashSet<ChunkPos>,

    glass_subtool: GlassChunkSubtool,
    cluster_subtool: ClusterSubtool,
    stall_subtool: StallChunkSubtool,
}
impl RehashTool {
    pub fn new() -> Self {
        let mut possible_rehash_chunks = HashMap::new();
        possible_rehash_chunks.insert(Dimension::Overworld, HashSet::new());
        possible_rehash_chunks.insert(Dimension::Nether, HashSet::new());
        possible_rehash_chunks.insert(Dimension::End, HashSet::new());

        let mut non_rehash_chunks = HashMap::new();
        non_rehash_chunks.insert(Dimension::Overworld, HashSet::new());
        non_rehash_chunks.insert(Dimension::Nether, HashSet::new());
        non_rehash_chunks.insert(Dimension::End, HashSet::new());

        Self {
            world_name: None,
            world_path: None,
            spawn_point: None,
            spawn_chunks: HashSet::new(),

            permanent_extra_chunks: HashSet::new(),
            temp_extra_chunks: HashSet::new(),

            possible_rehash_chunks,
            non_rehash_chunks,

            mask: 16383,
            target_cluster_size: 4200,
            dimension: Dimension::Overworld,
            view_origin: ChunkPos::new(311, -367),
            view_distance: 32,
            glass_chunk: ChunkPos::new(353, -370),
            rehash_chunk: ChunkPos::new(343, -398),
            selection_mode: None,

            loaded_chunks: HashSet::new(),

            glass_subtool: GlassChunkSubtool::new(),
            cluster_subtool: ClusterSubtool::new(),
            stall_subtool: StallChunkSubtool::new(),
        }
    }

    fn export_csv(&self, save_path: String) -> anyhow::Result<()> {
        let mut all_permaloaded_chunks: HashSet<ChunkPos> = HashSet::new();
        all_permaloaded_chunks.extend(&self.cluster_subtool.cluster_chunks);
        all_permaloaded_chunks.extend(&self.cluster_subtool.mst_chunks);
        all_permaloaded_chunks.extend(&self.permanent_extra_chunks);

        let permaload_chunks = all_permaloaded_chunks.iter().map(|chunk| format!("{},{}", chunk.x, chunk.z)).collect::<Vec<String>>();
        std::fs::write(&save_path, permaload_chunks.join("\n")).context(format!("Failed to write permaload chunks to file {}", &save_path))?;

        Ok(())
    }

    fn build_arborescence(&self) -> anyhow::Result<()> {
        let Some(world_path) = &self.world_path else {
            bail!("No world path set!");
        };

        let world = World::new(world_path);

        render_arborescence_world(&self.cluster_subtool.arborescence, &world, false)?;

        Ok(())
    }

    fn generate_arborescence_litematic(&self, save_path: String) -> anyhow::Result<()> {
        let mut litematica = LitematicaBuilder::new();

        render_arborescence_litematic(&self.cluster_subtool.arborescence, &mut litematica, false);

        litematica.save(&save_path, "cluster")?;

        println!("Origin: {:?}", litematica.get_origin());

        Ok(())
    }

    fn update_rehash_chunks(&mut self, state: &mut CommonState) {
        let layer_group = state.layers.get_layer_mut::<LayerGroup>("rehash").unwrap();

        let rehash_chunk_layer = layer_group.get_layer_mut::<HashSetLayer>("rehash_chunks").unwrap();
        let mut rehash_chunk_set = HashSet::new();
        rehash_chunk_set.extend(self.possible_rehash_chunks.get(&self.dimension).unwrap());
        rehash_chunk_layer.set_chunks(rehash_chunk_set);

        let non_rehash_chunk_layer = layer_group.get_layer_mut::<HashSetLayer>("non_rehash_chunks").unwrap();
        let mut non_rehash_chunk_set = HashSet::new();
        non_rehash_chunk_set.extend(self.non_rehash_chunks.get(&self.dimension).unwrap());
        non_rehash_chunk_layer.set_chunks(non_rehash_chunk_set);
    }

    fn update_view(&mut self, state: &mut CommonState) {
        let layer_group = state.layers.get_layer_mut::<LayerGroup>("rehash").unwrap();

        self.loaded_chunks.clear();

        let glass_chunk_layer = layer_group.get_layer_mut::<HashSetLayer>("glass_chunk").unwrap();
        let mut glass_chunk_set = HashSet::new();
        glass_chunk_set.insert(self.glass_chunk);
        self.loaded_chunks.insert(self.glass_chunk);
        glass_chunk_layer.set_chunks(glass_chunk_set);

        let view_origin_layer = layer_group.get_layer_mut::<HashSetLayer>("view_origin").unwrap();
        let mut view_origin_set = HashSet::new();
        view_origin_set.insert(self.view_origin);
        self.loaded_chunks.insert(self.view_origin);
        view_origin_layer.set_chunks(view_origin_set);

        let rehash_chunk_layer = layer_group.get_layer_mut::<HashSetLayer>("rehash_chunk").unwrap();
        let mut rehash_chunk_set = HashSet::new();
        rehash_chunk_set.insert(self.rehash_chunk);
        self.loaded_chunks.insert(self.rehash_chunk);
        rehash_chunk_layer.set_chunks(rehash_chunk_set);

        let valid_glass_chunks_layer = layer_group.get_layer_mut::<VirtualChunkLayer<ValidGlassChunkProvider>>("valid_glass_chunks").unwrap();
        valid_glass_chunks_layer.provider.set_mask(self.mask);
        valid_glass_chunks_layer.provider.set_target_cluster_size(self.target_cluster_size);
        valid_glass_chunks_layer.provider.set_tolerance(self.glass_subtool.tolerance);
        valid_glass_chunks_layer.provider.set_enabled(self.glass_subtool.enabled);

        let valid_stall_chunks_layer = layer_group.get_layer_mut::<VirtualChunkLayer<ValidStallChunkProvider>>("valid_stall_chunks").unwrap();
        valid_stall_chunks_layer.provider.set_enabled(self.stall_subtool.enabled);
        valid_stall_chunks_layer.provider.set_anvil_mask(self.stall_subtool.anvil_mask);
        valid_stall_chunks_layer.provider.set_rehash_chunk(self.rehash_chunk);

        let possible_cluster_chunks_layer = layer_group.get_layer_mut::<HashSetLayer>("possible_cluster_chunks").unwrap();
        let mut possible_cluster_chunk_set = HashSet::new();
        possible_cluster_chunk_set.extend(&self.cluster_subtool.possible_cluster_chunks);
        possible_cluster_chunks_layer.set_chunks(possible_cluster_chunk_set);

        let mst_chunk_layer = layer_group.get_layer_mut::<HashSetLayer>("mst_chunk").unwrap();
        let mut mst_chunk_set = HashSet::new();
        mst_chunk_set.extend(&self.cluster_subtool.mst_chunks);
        self.loaded_chunks.extend(&mst_chunk_set);
        mst_chunk_layer.set_chunks(mst_chunk_set);

        let cluster_chunk_layer = layer_group.get_layer_mut::<HashSetLayer>("cluster_chunks").unwrap();
        let mut cluster_chunk_set = HashSet::new();
        cluster_chunk_set.extend(&self.cluster_subtool.cluster_chunks);
        self.loaded_chunks.extend(&cluster_chunk_set);
        cluster_chunk_layer.set_chunks(cluster_chunk_set);

        let permanent_extra_chunk_layer = layer_group.get_layer_mut::<HashSetLayer>("permanent_extra_chunks").unwrap();
        let mut permanent_extra_chunk_set = HashSet::new();
        permanent_extra_chunk_set.extend(&self.permanent_extra_chunks);
        self.loaded_chunks.extend(&permanent_extra_chunk_set);
        permanent_extra_chunk_layer.set_chunks(permanent_extra_chunk_set);

        let temporary_extra_chunk_layer = layer_group.get_layer_mut::<HashSetLayer>("temporary_extra_chunks").unwrap();
        let mut temporary_extra_chunk_set = HashSet::new();
        temporary_extra_chunk_set.extend(&self.temp_extra_chunks);
        self.loaded_chunks.extend(&temporary_extra_chunk_set);
        temporary_extra_chunk_layer.set_chunks(temporary_extra_chunk_set);

        let stall_chunk_layer = layer_group.get_layer_mut::<HashSetLayer>("stall_chunks").unwrap();
        let mut stall_chunk_set = HashSet::new();
        stall_chunk_set.extend(&self.stall_subtool.stall_chunks);
        self.loaded_chunks.extend(&stall_chunk_set);
        stall_chunk_layer.set_chunks(stall_chunk_set);

        let view_distance_layer = layer_group.get_layer_mut::<HashSetLayer>("view_distance").unwrap();
        let mut view_distance_set = HashSet::new();
        let nw_corner = ChunkPos::new(self.view_origin.x - self.view_distance, self.view_origin.z - self.view_distance);
        let se_corner = ChunkPos::new(self.view_origin.x + self.view_distance, self.view_origin.z + self.view_distance);
        let lower_x = min(nw_corner.x, se_corner.x);
        let lower_z = min(nw_corner.z, se_corner.z);
        let upper_x = max(nw_corner.x, se_corner.x);
        let upper_z = max(nw_corner.z, se_corner.z);

        for x in lower_x..=upper_x {
            for z in lower_z..=upper_z {
                view_distance_set.insert(ChunkPos::new(x, z));
                self.loaded_chunks.insert(ChunkPos::new(x, z));
            }
        }
        view_distance_layer.set_chunks(view_distance_set);

        let spawn_chunks_layer = layer_group.get_layer_mut::<HashSetLayer>("spawn_chunks").unwrap();
        let mut spawn_chunks_set = HashSet::new();
        if self.dimension == Dimension::Overworld {
            spawn_chunks_set.extend(&self.spawn_chunks);
        }
        self.loaded_chunks.extend(&spawn_chunks_set);
        spawn_chunks_layer.set_chunks(spawn_chunks_set);

        state.sidebar.show();
        state.sidebar.set_mask(self.mask);
        let natural_hash = self.glass_chunk.hash(self.mask);
        let clustered_hash = (natural_hash + self.target_cluster_size as i32) & self.mask; // TODO: Use cluster if available
        state.sidebar.set_hashes(natural_hash, clustered_hash);
    }

    fn load_dimension(&mut self, world_path: &String, dimension: Dimension, task_list: &mut TaskList<State>) {
        let world_path = world_path.clone();
        let loading_task = Task::start_progress(move |tx| {
            let world = World::new(&world_path);

            let total_regions = world.get_num_regions(dimension)?;
            let region_count = AtomicUsize::new(0);

            let result = world.region_pos_iter(dimension)?.par_bridge().flat_map(|region_pos| -> anyhow::Result<(Vec<ChunkPos>, Vec<ChunkPos>)> {
                let mut region_rehash_chunks: Vec<ChunkPos> = Vec::new();
                let mut region_non_rehash_chunks: Vec<ChunkPos> = Vec::new();

                let Some(region) = world.get_region_uncached(region_pos, dimension)? else {
                    bail!("Failed to load region at {region_pos:?} in dimension {dimension:?}");
                };

                for (chunk_pos, chunk) in region.chunk_iter() {
                    match dimension {
                        Dimension::Overworld => {
                            if chunk.num_subchunks() <= 4 {
                                region_rehash_chunks.push(chunk_pos.offset(region_pos.into()));
                            } else {
                                region_non_rehash_chunks.push(chunk_pos.offset(region_pos.into()));
                            }
                        }
                        Dimension::Nether => {
                            if chunk.num_subchunks() <= 8 {
                                region_rehash_chunks.push(chunk_pos.offset(region_pos.into()));
                            } else {
                                region_non_rehash_chunks.push(chunk_pos.offset(region_pos.into()));
                            }
                        }
                        Dimension::End => {
                            if chunk.num_subchunks() == 0 {
                                region_rehash_chunks.push(chunk_pos.offset(region_pos.into()));
                            } else {
                                region_non_rehash_chunks.push(chunk_pos.offset(region_pos.into()));
                            }
                        }
                    }
                }

                let current_regions = region_count.fetch_add(1, Ordering::SeqCst) + 1;
                let progress = current_regions as f32 / total_regions as f32;
                tx.send(TaskStatus::Progress(progress))?;

                Ok((region_rehash_chunks, region_non_rehash_chunks))
            }).reduce(|| (Vec::new(), Vec::new()), |mut acc, res| {
                let (rehash, non_rehash) = res;
                acc.0.extend(rehash);
                acc.1.extend(non_rehash);
                acc
            });
            Ok(result)
        }, move |result: anyhow::Result<(Vec<ChunkPos>, Vec<ChunkPos>)>, state: &mut State| {
            match result {
                Ok((rehash, non_rehash)) => {
                    if let Some(tool) = state.toolbox.get_current_tool_mut::<RehashTool>() {
                        let rehash_set = HashSet::from_iter(rehash);
                        let non_rehash_set = HashSet::from_iter(non_rehash);
                        tool.possible_rehash_chunks.insert(dimension, rehash_set);
                        tool.non_rehash_chunks.insert(dimension, non_rehash_set);

                        tool.update_rehash_chunks(&mut state.common_state);
                        tool.update_view(&mut state.common_state);
                    } else {
                        println!("RehashTool: Tool switched before task finished!");
                    }
                }
                Err(err) => {
                    println!("Error loading {dimension:?}: {err}");
                    message_box_ok("Error", &format!("Error loading {dimension:?}, check console"), MessageBoxIcon::Error)
                }
            }
        });

        task_list.add_task(&format!("Loading {dimension:?}"), loading_task);
    }

    fn load_world(&mut self, world_path: &String, task_list: &mut TaskList<State>) {
        let world = World::new(world_path);
        let level_dat = world.get_level_dat().unwrap();
        self.spawn_point = Some(level_dat.spawn);
        if let Ok(spawn_chunks) = world.get_spawn_chunks() {
            self.spawn_chunks = HashSet::from_iter(spawn_chunks);
        }

        self.load_dimension(world_path, Dimension::Overworld, task_list);
        self.load_dimension(world_path, Dimension::Nether, task_list);
        self.load_dimension(world_path, Dimension::End, task_list);
    }

    fn calculate_cluster(&mut self, task_list: &mut TaskList<State>) {
        let mask = self.mask;
        let cluster_target = self.glass_chunk;
        let load_origin = self.view_origin;
        let target_cluster_size = self.target_cluster_size;
        let permanent_extra_chunks = self.permanent_extra_chunks.clone();
        // let possible_cluster_chunks = self.cluster_subtool.possible_cluster_chunks.clone();
        let mut possible_cluster_chunks = IndexSet::new();
        possible_cluster_chunks.extend(&self.cluster_subtool.possible_cluster_chunks);
        let cluster_task = Task::start(move || {
            let mut cluster_finder = RehashClusterFinder::new(mask, load_origin, cluster_target, false);
            for chunk_pos in &possible_cluster_chunks {
                cluster_finder.add_chunk(*chunk_pos);
            }
            let cluster = cluster_finder.cluster_for(cluster_target, target_cluster_size);
            let mut cluster_set = IndexSet::new();
            cluster_set.extend(&cluster);

            let mut arborescence_set = IndexSet::new();
            arborescence_set.extend(&cluster);
            arborescence_set.extend(&permanent_extra_chunks);
            // cluster_set.extend(permanent_extra_chunks);

            let arborescence = compute_arborescence(&arborescence_set, load_origin);
            let mst_set = render_arborescence_set(&arborescence);

            (cluster_set, mst_set, arborescence)
        }, |result, state: &mut State| {
            if let Some(tool) = state.toolbox.get_current_tool_mut::<RehashTool>() {
                tool.cluster_subtool.cluster_chunks = result.0;
                tool.cluster_subtool.mst_chunks = result.1;
                tool.cluster_subtool.arborescence = result.2;
                tool.update_view(&mut state.common_state);
            } else {
                println!("RehashTool: Tool switched before task finished!");
            }
        });

        task_list.add_task("Calculating Cluster", cluster_task);
    }
}
impl Tool for RehashTool {
    fn start(&mut self, state: &mut CommonState) {
        let mut layer_group = LayerGroup::new();

        layer_group.add_layer("rehash_chunks", HashSetLayer::new(HashSet::new(), Color::from_rgb(128, 128, 128)), 0);
        layer_group.add_layer("non_rehash_chunks", HashSetLayer::new(HashSet::new(), Color::from_rgb(0xf6, 0xfa, 0xbd)), 1);

        layer_group.add_layer("permanent_extra_chunks", HashSetLayer::new(HashSet::new(), Color::from_rgb(0xcc, 0x55, 0x00)), 2);
        layer_group.add_layer("temporary_extra_chunks", HashSetLayer::new(HashSet::new(), Color::from_rgb(0xff, 0xac, 0x1c)), 3);

        layer_group.add_layer("view_distance", HashSetLayer::new(HashSet::new(), Color::from_rgba(0, 128, 0, 230)), 4);
        layer_group.add_layer("spawn_chunks", HashSetLayer::new(HashSet::new(), Color::from_rgba(0, 96, 0, 230)), 5);

        layer_group.add_layer("possible_cluster_chunks", HashSetLayer::new(HashSet::new(), Color::from_rgba(0, 0, 255, 128)), 6);
        layer_group.add_layer("mst_chunk", HashSetLayer::new(HashSet::new(), Color::from_rgb(100, 255, 100)), 7);
        layer_group.add_layer("cluster_chunks", HashSetLayer::new(HashSet::new(), Color::BLUE), 8);

        layer_group.add_layer("view_origin", HashSetLayer::new(HashSet::new(), Color::MAGENTA), 9);

        layer_group.add_layer("stall_chunks", HashSetLayer::new(HashSet::new(), Color::from_rgb(128, 0, 0)), 10);
        layer_group.add_layer("glass_chunk", HashSetLayer::new(HashSet::new(), Color::CYAN), 11);
        layer_group.add_layer("valid_glass_chunks", VirtualChunkLayer::new(ValidGlassChunkProvider::new(self.mask, self.target_cluster_size, self.glass_subtool.tolerance)), 12);

        layer_group.add_layer("rehash_chunk", HashSetLayer::new(HashSet::new(), Color::RED), 13);
        layer_group.add_layer("valid_stall_chunks", VirtualChunkLayer::new(ValidStallChunkProvider::new()), 14);

        state.layers.add_layer("rehash", layer_group, 0);

        self.update_view(state);
    }

    fn stop(&mut self, state: &mut CommonState) {
        state.layers.remove_layer("rehash");
    }

    fn gui(&mut self, state: &mut CommonState, task_list: &mut TaskList<State>, gui_ctx: &GuiContext) {
        egui::Window::new("Rehash").show(gui_ctx, |ui| {
            egui::Grid::new("rehash_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    if ui.button("Load World").clicked() {
                        let world_path = tinyfiledialogs::select_folder_dialog("Open Minecraft World", "");
                        if let Some(world_path) = world_path {
                            self.world_name = Some(String::from(Path::new(&world_path).components().last().unwrap().as_os_str().to_string_lossy()));
                            self.world_path = Some(world_path.clone());
                            self.load_world(&world_path, task_list);
                        }
                    }
                    if let Some(world_name) = &self.world_name {
                        ui.label(world_name);
                    }
                    ui.end_row();

                    ui.label("Dimension");
                    egui::ComboBox::new("dimension", "")
                        .selected_text(format!("{:?}", self.dimension))
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut self.dimension, Dimension::Overworld, "Overworld").clicked() ||
                                ui.selectable_value(&mut self.dimension, Dimension::Nether, "Nether").clicked() ||
                                ui.selectable_value(&mut self.dimension, Dimension::End, "End").clicked() {
                                self.update_rehash_chunks(state);
                                self.update_view(state);
                            }
                        });
                    ui.end_row();

                    ui.label("Hashmap Size");
                    egui::ComboBox::new("hashmap_size", "")
                        .selected_text(format!("{}", self.mask + 1))
                        .show_ui(ui, |ui| {
                            if ui.selectable_value(&mut self.mask, 2047, "2048").clicked() ||
                                ui.selectable_value(&mut self.mask, 4095, "4096").clicked() ||
                                ui.selectable_value(&mut self.mask, 8191, "8192").clicked() ||
                                ui.selectable_value(&mut self.mask, 16383, "16384").clicked() {
                                self.cluster_subtool.invalidate_cluster();
                                self.update_view(state);
                            }
                        });
                    ui.end_row();

                    ui.label("Target Cluster Size");
                    if ui.add(egui::Slider::new(&mut self.target_cluster_size, 1..=10000)).changed() {
                        self.cluster_subtool.invalidate_cluster();
                        self.update_view(state);
                    }
                    ui.end_row();

                    ui.label("View Origin");
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut self.view_origin.x)).changed() {
                            self.update_view(state);
                        }
                        if ui.add(egui::DragValue::new(&mut self.view_origin.z)).changed() {
                            self.update_view(state);
                        }
                        if let Some(SelectionMode::ViewOrigin) = self.selection_mode {
                            ui.add_enabled(false, egui::Button::new("⛶"));
                        } else {
                            if ui.button("⛶").clicked() {
                                self.selection_mode = Some(SelectionMode::ViewOrigin);
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("View Distance");
                    if ui.add(egui::Slider::new(&mut self.view_distance, 1..=32)).changed() {
                        self.update_view(state);
                    }
                    ui.end_row();

                    ui.label("Glass Chunk");
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut self.glass_chunk.x)).changed() {
                            self.update_view(state);
                        }
                        if ui.add(egui::DragValue::new(&mut self.glass_chunk.z)).changed() {
                            self.update_view(state);
                        }
                        if let Some(SelectionMode::GlassChunk) = self.selection_mode {
                            ui.add_enabled(false, egui::Button::new("⛶"));
                        } else {
                            if ui.button("⛶").clicked() {
                                self.selection_mode = Some(SelectionMode::GlassChunk);
                            }
                        }
                    });
                    ui.end_row();

                    ui.label("Rehash Chunk");
                    ui.horizontal(|ui| {
                        if ui.add(egui::DragValue::new(&mut self.rehash_chunk.x)).changed() {
                            self.update_view(state);
                        }
                        if ui.add(egui::DragValue::new(&mut self.rehash_chunk.z)).changed() {
                            self.update_view(state);
                        }
                        if let Some(SelectionMode::RehashChunk) = self.selection_mode {
                            ui.add_enabled(false, egui::Button::new("⛶"));
                        } else {
                            if ui.button("⛶").clicked() {
                                self.selection_mode = Some(SelectionMode::RehashChunk);
                            }
                        }
                    });
                    ui.end_row();
                });

            ui.collapsing("Glass Chunk Selector", |ui| {
                egui::Grid::new("rehash_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Enabled");
                        if ui.checkbox(&mut self.glass_subtool.enabled, "").changed() {
                            self.update_view(state);
                        }
                        ui.end_row();

                        ui.label("Tolerance");
                        if ui.add(egui::Slider::new(&mut self.glass_subtool.tolerance, 0..=1000)).changed() {
                            self.update_view(state);
                        }
                        ui.end_row();
                    });
            });

            ui.collapsing("Cluster Selector", |ui| {
                egui::Grid::new("cluster_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        if ui.button("Set Search Area").clicked() {
                            self.cluster_subtool.possible_cluster_chunks.clear();
                            self.cluster_subtool.possible_cluster_chunks.extend(state.selection.clone());
                            self.cluster_subtool.invalidate_cluster();
                            self.calculate_cluster(task_list);
                        }
                        ui.label(format!("{} chunks", self.cluster_subtool.possible_cluster_chunks.len()));
                        ui.end_row();

                        if ui.button("Calculate Cluster").clicked() {
                            self.calculate_cluster(task_list);
                        }
                        let cluster_size = self.cluster_subtool.cluster_chunks.len();
                        let cluster_size_str = format!("{}", cluster_size);
                        if cluster_size != self.target_cluster_size {
                            ui.label(RichText::new(cluster_size_str).color(Color32::RED));
                        } else {
                            ui.label(cluster_size_str);
                        }
                        ui.end_row();
                    });
            });

            ui.collapsing("Stall Chunk Selector", |ui| {
                egui::Grid::new("stall_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Enabled");
                        if ui.checkbox(&mut self.stall_subtool.enabled, "").changed() {
                            self.update_view(state);
                        }
                        ui.end_row();

                        if ui.button("Set Stall Chunks").clicked() {
                            self.stall_subtool.stall_chunks.clear();
                            self.stall_subtool.stall_chunks.extend(&state.selection);
                            self.update_view(state);
                        }
                        ui.label(format!("{} chunks", self.stall_subtool.stall_chunks.len()));
                        ui.end_row();

                        ui.label("Anvil Hashmap Size");
                        egui::ComboBox::new("anvil_mask", "")
                            .selected_text(format!("{:?}", self.stall_subtool.anvil_mask + 1))
                            .show_ui(ui, |ui| {
                                let mut cur_size = 16;
                                for _ in 0..10 {
                                    if ui.selectable_value(&mut self.stall_subtool.anvil_mask, cur_size - 1, format!("{}", cur_size)).clicked() {
                                        self.update_view(state);
                                    }
                                    cur_size *= 2;
                                }
                            });
                        ui.end_row();

                        ui.label("Rehash Chunk Bin");
                        ui.label(format!("{:?}", bin_for(self.rehash_chunk, self.stall_subtool.anvil_mask)));
                        ui.end_row();
                    });
            });

            ui.collapsing("Extra Chunk Selection", |ui| {
                egui::Grid::new("extra_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        if ui.button("Set Permanent Extras").clicked() {
                            self.permanent_extra_chunks.clear();
                            self.permanent_extra_chunks.extend(&state.selection);
                            self.cluster_subtool.invalidate_cluster();
                            self.update_view(state);
                        }
                        ui.label(format!("{} chunks", self.permanent_extra_chunks.len()));
                        ui.end_row();

                        if ui.button("Set Temporary Extras").clicked() {
                            self.temp_extra_chunks.clear();
                            self.temp_extra_chunks.extend(&state.selection);
                            self.update_view(state);
                        }
                        ui.label(format!("{} chunks", self.temp_extra_chunks.len()));
                        ui.end_row();
                    })
            });

            egui::Grid::new("footer_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Total Loaded Chunks");
                    let total_chunks = self.loaded_chunks.len();
                    let upsize_limit = ((self.mask + 1) as usize * 3) / 4;
                    let loaded_chunks_str = format!("{}/{}", total_chunks, upsize_limit);
                    if total_chunks > upsize_limit {
                        ui.label(RichText::new(loaded_chunks_str).color(Color32::RED));
                    } else {
                        ui.label(loaded_chunks_str);
                    }
                    ui.end_row();

                    if ui.button("Export CSV").clicked() {
                        let save_path = tinyfiledialogs::save_file_dialog_with_filter("Export Location", "", &["*.csv"], ".csv");
                        if let Some(save_path) = save_path {
                            if let Err(err) = self.export_csv(save_path) {
                                eprintln!("Error exporting CSV: {}", err);
                                message_box_ok("Error", &format!("Error exporting CSV: {}", err), MessageBoxIcon::Error)
                            }
                        }
                    }
                    ui.end_row();

                    if ui.button("Build").clicked() {
                        let answer = tinyfiledialogs::message_box_yes_no("Are you sure?", "This will build the cluster and extra chunk loader directly in the world. It may take a while. Ensure you have backed up the world first.", MessageBoxIcon::Warning, YesNo::No);
                        if answer == YesNo::Yes {
                            if let Err(err) = self.build_arborescence() {
                                eprintln!("Error building arborescence: {}", err);
                                message_box_ok("Error", &format!("Error building arborescence: {}", err), MessageBoxIcon::Error)
                            }
                        }
                    }
                    ui.end_row();

                    if ui.button("Generate Litematic").clicked() {
                        let save_path = tinyfiledialogs::save_file_dialog_with_filter("Save Location", "", &["*.litematic"], ".litematic");

                        if let Some(save_path) = save_path {
                            if let Err(err) = self.generate_arborescence_litematic(save_path) {
                                eprintln!("Error generating litematic: {}", err);
                                message_box_ok("Error", &format!("Error generating litematic: {}", err), MessageBoxIcon::Error)
                            }
                        }
                    }
                })
        });
    }

    fn on_chunk_selected(&mut self, chunk: ChunkPos, state: &mut CommonState) {
        match self.selection_mode {
            None => {},
            Some(SelectionMode::GlassChunk) => {
                self.glass_chunk = chunk;
                self.cluster_subtool.invalidate_cluster();
                self.update_view(state);
            },
            Some(SelectionMode::ViewOrigin) => {
                self.view_origin = chunk;
                self.cluster_subtool.invalidate_cluster();
                self.update_view(state);
            }
            Some(SelectionMode::RehashChunk) => {
                self.rehash_chunk = chunk;
                self.update_view(state);
            }
        }
        self.selection_mode = None;
    }
}
