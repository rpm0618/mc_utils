use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use anyhow::{bail, Context, Result};
use ggegui::{egui, GuiContext};
use ggez::graphics::Color;
use mc_utils::positions::ChunkPos;
use mc_utils::world::Dimension;
use crate::chunk_viewer::chunk_layer::{CheckerboardProvider, ChunkColor, LayerGroup, VirtualChunkLayer, VirtualChunkProvider};
use crate::chunk_viewer::event_handler::{CommonState, State};
use crate::chunk_viewer::task_list::TaskList;
use crate::chunk_viewer::tools::Tool;
use crate::chunk_viewer::viewport::Viewport;

use base64::prelude::*;
use serde::Deserialize;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum Event {
    Loaded,
    Generated,
    Populated,
    UnloadScheduled,
    Unloaded,
    AlreadyLoaded
}
impl Event {
    fn color(&self) -> Color {
        match self {
            Event::Loaded => Color::from_rgb(19, 232, 232),
            Event::Generated => Color::from_rgb(120, 67, 21),
            Event::Populated => Color::from_rgb(234, 54, 128),
            Event::UnloadScheduled => Color::from_rgb(221, 221, 0),
            Event::Unloaded => Color::from_rgb(223, 48, 48),
            Event::AlreadyLoaded => Color::from_rgb(44, 103, 152),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
struct Metadata {
    #[serde(rename = "stackTrace")]
    stack_trace: Option<String>,
    custom: Option<String>,
}

#[derive(Debug, Clone)]
struct ChunkDebugEntry {
    position: ChunkPos,
    order: i32,
    event: Event,
    metadata: Metadata
}
#[derive(Debug, Clone)]
struct EntryHolder {
    tick: u32,
    total_size: u32,
    entries: HashMap<ChunkPos, Vec<ChunkDebugEntry>>
}
impl EntryHolder {
    fn new(tick: u32) -> Self {
        Self {
            tick,
            total_size: 0,
            entries: HashMap::new()
        }
    }

    fn add_entry(&mut self, entry: ChunkDebugEntry) {
        if !self.entries.contains_key(&entry.position) {
            self.entries.insert(entry.position, Vec::new());
        }
        self.entries.get_mut(&entry.position).unwrap().push(entry);
        self.total_size += 1;
    }
}
struct ChunkDebugProvider {
    entry_holder: Option<EntryHolder>,
    dirty: bool
}
impl ChunkDebugProvider {
    fn new() -> Self {
        Self {
            entry_holder: None,
            dirty: false
        }
    }

    fn set_entry_holder(&mut self, entry_holder: Option<EntryHolder>) {
        self.entry_holder = entry_holder;
        self.dirty = true;
    }
}
impl VirtualChunkProvider for ChunkDebugProvider {
    fn chunks_in_viewport(&self, _viewport: &Viewport) -> impl Iterator<Item=(ChunkPos, impl Into<ChunkColor>)> {
        let entry_holder = self.entry_holder.as_ref().map_or(EntryHolder::new(0), |eh| eh.clone());
        entry_holder.entries.into_iter().map(|(position, entries)| {
            let index = entries.len() - 1;
            let primary = entries[index].event.color();
            let secondary = if index > 0 {
                Some(entries[index - 1].event.color())
            } else {
                None
            };
            let chunk_color = ChunkColor::new(primary, secondary);
            (position, chunk_color)
        })
    }

    fn check_dirty(&mut self) -> bool {
        let result = self.dirty;
        self.dirty = false;
        result
    }
}

pub struct ChunkDebugTool {
    entries: HashMap<Dimension, Vec<EntryHolder>>,

    current_dimension: Option<Dimension>,
    current_entry_index: Option<usize>
}
impl ChunkDebugTool {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            current_dimension: None,
            current_entry_index: None
        }
    }

    fn load_dump(&mut self, state: &mut CommonState, dump_path: String) -> Result<()> {
        self.entries.clear();
        self.entries.insert(Dimension::Overworld, Vec::new());
        self.entries.insert(Dimension::Nether, Vec::new());
        self.entries.insert(Dimension::End, Vec::new());

        let mut current_entry_holders: HashMap<Dimension, EntryHolder> = HashMap::new();

        let file = File::open(dump_path)?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            let parts: Vec<_> = line.split(",").collect();
            let position = ChunkPos::new(parts[0].parse()?, parts[1].parse()?);
            let tick: u32 = parts[2].parse()?;
            let dimension = match parts[3] {
                "0" => Dimension::Overworld,
                "1" => Dimension::Nether,
                "2" => Dimension::End,
                _ => bail!("Unexpected dimension {}", parts[3])
            };
            let event = match parts[4] {
                "LOADED" => Event::Loaded,
                "GENERATED" => Event::Generated,
                "POPULATED" => Event::Populated,
                "UNLOAD_SCHEDULED" => Event::UnloadScheduled,
                "UNLOADED" => Event::Unloaded,
                "ALREADY_LOADED" => Event::AlreadyLoaded,
                _ => bail!("Unexpected event {}", parts[4])
            };
            let metadata = parts[5].to_string();
            let metadata = BASE64_STANDARD.decode(metadata)?;
            let metadata = serde_json::from_slice(&metadata)?;

            let mut entry = ChunkDebugEntry {
                position,
                order: 0,
                event,
                metadata
            };

            if !current_entry_holders.contains_key(&dimension) {
                current_entry_holders.insert(dimension, EntryHolder::new(tick));
            }

            let entry_holder = current_entry_holders.get(&dimension).context("?")?;
            if tick != entry_holder.tick {
                let old_entry_holder = current_entry_holders.remove(&dimension).context("?")?;
                self.entries.get_mut(&dimension).context("?")?.push(old_entry_holder);

                let mut new_entry_holder = EntryHolder::new(tick);
                new_entry_holder.add_entry(entry);
                current_entry_holders.insert(dimension, new_entry_holder);
            } else {
                let entry_holder = current_entry_holders.get_mut(&dimension).context("?")?;
                entry.order = entry_holder.total_size as i32;
                entry_holder.add_entry(entry)
            }
        }
        for dimension in [Dimension::Overworld, Dimension::Nether, Dimension::End] {
            if let Some(entry_holder) = current_entry_holders.remove(&dimension) {
                self.entries.get_mut(&dimension).context("?")?.push(entry_holder);
            }
        }
        self.propagate_already_loaded()?;
        self.current_dimension = Some(Dimension::Overworld);
        self.on_dimension_changed(state)?;
        Ok(())
    }

    fn propagate_already_loaded(&mut self) -> Result<()> {
        for dimension in [Dimension::Overworld, Dimension::Nether, Dimension::End] {
            if let Some(entry_holders) = self.entries.get_mut(&dimension) {
                for i in 1..entry_holders.len() {
                    let (start, end) = entry_holders.split_at_mut(i);
                    let entry_holder = &mut end[0];
                    let prev_entry_holder = &start[i - 1];

                    for (position, prev_entries) in &prev_entry_holder.entries {
                        let last = prev_entries.last().context("?")?;
                        if last.event != Event::Unloaded && !entry_holder.entries.contains_key(&position) {
                            let event = if last.event == Event::UnloadScheduled {
                                Event::UnloadScheduled
                            } else {
                                Event::AlreadyLoaded
                            };
                            let already_loaded_entry = ChunkDebugEntry {
                                position: *position,
                                order: -1,
                                event,
                                metadata: Metadata::default()
                            };
                            let mut entries = Vec::new();
                            entries.push(already_loaded_entry);
                            entry_holder.entries.insert(*position, entries);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn get_current_entry_holder(&self) -> Option<&EntryHolder> {
        self.current_dimension.map(|dimension| {
            self.current_entry_index.map(|index| {
                self.entries.get(&dimension).map(|entries| entries.get(index)).flatten()
            }).flatten()
        }).flatten()
    }

    fn update_layers(&mut self, state: &mut CommonState) -> Result<()> {
        let entry_holder = self.get_current_entry_holder().map(|eh| eh.clone());
        let cdb_layer = state.layers.get_layer_mut::<LayerGroup>("chunk_debug").context("?")?;
        let debug_layer = cdb_layer.get_layer_mut::<VirtualChunkLayer<CheckerboardProvider<ChunkDebugProvider>>>("debug").context("?")?;
        debug_layer.provider.provider.set_entry_holder(entry_holder);
        Ok(())
    }

    fn on_dimension_changed(&mut self, state: &mut CommonState) -> Result<()> {
        self.current_entry_index = None;
        if let Some(dimension) = self.current_dimension {
            if let Some(entries) = self.entries.get(&dimension) {
                if !entries.is_empty() {
                    self.current_entry_index = Some(0);
                }
            }
        }
        self.update_layers(state)?;
        Ok(())
    }

    fn step_home(&mut self, state: &mut CommonState) -> Result<()> {
        if self.current_entry_index.is_some() {
            self.current_entry_index = Some(0);
            self.update_layers(state)?;
        }
        Ok(())
    }
    fn step_back(&mut self, state: &mut CommonState) -> Result<()> {
        if let Some(index) = self.current_entry_index {
            if index > 0 {
                self.current_entry_index = Some(index - 1);
                self.update_layers(state)?;
            }
        }
        Ok(())
    }
    fn step_forward(&mut self, state: &mut CommonState) -> Result<()> {
        if let Some(index) = self.current_entry_index {
            if self.current_dimension.is_some() && self.entries.contains_key(&self.current_dimension.context("?")?) {
                let entries = self.entries.get(&self.current_dimension.unwrap()).context("?")?;
                if index < entries.len() - 1 {
                    self.current_entry_index = Some(index + 1);
                    self.update_layers(state)?
                }
            }
        }
        Ok(())
    }
    fn step_end(&mut self, state: &mut CommonState) -> Result<()> {
        if let Some(_) = self.current_entry_index {
            if self.current_dimension.is_some() && self.entries.contains_key(&self.current_dimension.context("?")?) {
                let entries = self.entries.get(&self.current_dimension.unwrap()).context("?")?;
                self.current_entry_index = Some(entries.len() - 1);
                self.update_layers(state)?
            }
        }
        Ok(())
    }
}
impl Tool for ChunkDebugTool {
    fn start(&mut self, state: &mut CommonState) {
        let mut layer_group = LayerGroup::new();
        let provider = CheckerboardProvider::new(ChunkDebugProvider::new(), Color::from_rgb(20, 20, 20));
        layer_group.add_layer("debug", VirtualChunkLayer::new(provider), 0);
        state.layers.add_layer("chunk_debug", layer_group, 0);
    }

    fn stop(&mut self, state: &mut CommonState) {
        state.layers.remove_layer("chunk_debug");
    }

    fn gui(&mut self, state: &mut CommonState, _task_list: &mut TaskList<State>, gui_ctx: &GuiContext) {
        egui::Window::new("1.8 Chunk Debug").show(gui_ctx, |ui| {
            egui::Grid::new("chunk_debug_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    if ui.button("Open Dump").clicked() {
                        let dump_path = tinyfiledialogs::open_file_dialog("Open", "", Some((&["*.csv"], ".csv")));
                        if let Some(dump_path) = dump_path {
                            self.load_dump(state, dump_path).unwrap();
                        }
                    }
                    ui.end_row();

                    ui.label("Current Dimension");
                    egui::ComboBox::new("current_dimension", "")
                        .selected_text(self.current_dimension.map_or("".to_string(), |d| format!("{:?}", d)))
                        .show_ui(ui, |ui| {
                            for dimension in [Dimension::Overworld, Dimension::Nether, Dimension::End] {
                                if ui.selectable_value(&mut self.current_dimension, Some(dimension), format!("{:?}", dimension)).clicked() {
                                    self.on_dimension_changed(state).unwrap();
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Current Tick");
                    egui::ComboBox::new("current_tick", "")
                        .selected_text(self.get_current_entry_holder().map_or("".to_string(), |eh| format!("{}", eh.tick)))
                        .show_ui(ui, |ui| {
                            let default = Vec::new();
                            let entry_holders = if self.current_dimension.is_some() && self.entries.contains_key(&self.current_dimension.unwrap()) {
                                self.entries.get(&self.current_dimension.unwrap()).unwrap()
                            } else {
                                &default
                            };
                            for (i, eh) in entry_holders.clone().iter().enumerate() {
                                if ui.selectable_value(&mut self.current_entry_index, Some(i), format!("{}", eh.tick)).clicked() {
                                    self.update_layers(state).unwrap();
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Controls");
                    ui.horizontal(|ui| {
                        if ui.button("|<").clicked() {
                            self.step_home(state).unwrap();
                        }
                        if ui.button("<").clicked() {
                            self.step_back(state).unwrap();
                        }
                        if ui.button(">").clicked() {
                            self.step_forward(state).unwrap();
                        }
                        if ui.button(">|").clicked() {
                            self.step_end(state).unwrap();
                        }
                    });
                    ui.end_row();
                });

            if state.selection.len() == 1 {
                let chunk = state.selection.iter().next().unwrap();
                if let Some(entry_holder) = self.get_current_entry_holder() {
                    if let Some(entries) = entry_holder.entries.get(chunk) {
                        let mut display = String::new();
                        for entry in entries {
                            display.push_str(&format!("Event: {:?}, Order: {}\n", entry.event, entry.order));
                            if let Some(custom) = &entry.metadata.custom {
                                display.push_str(&format!("Custom: {}\n", custom));
                            }
                            if let Some(stack_trace) = &entry.metadata.stack_trace {
                                display.push_str(&format!("Stack Trace: {}\n", stack_trace));
                            }
                            display.push_str("\n");
                        }
                        egui::ScrollArea::both().show(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.label(display);
                        });
                    }
                }
            }
        });
    }
}