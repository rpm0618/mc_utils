use std::cell::RefCell;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::rc::Rc;
use anyhow::{Result};
use ggegui::{egui, GuiContext};
use ggez::graphics::Color;
use mc_utils::world::Dimension;
use crate::chunk_viewer::chunk_layer::{CheckerboardProvider, LayerGroup, VirtualChunkLayer};
use crate::chunk_viewer::event_handler::{CommonState, State};
use crate::chunk_viewer::task_list::{TaskList};
use crate::chunk_viewer::tools::chunk_debug::{ChunkDebugEntry};
use crate::chunk_viewer::tools::chunk_debug::map::ChunkDebugMap;
use crate::chunk_viewer::tools::chunk_debug::server::{ChunkDebugServer, ServerStatus};
use crate::chunk_viewer::tools::Tool;

pub struct ChunkDebugTool {
    chunk_debug_map: Rc<RefCell<ChunkDebugMap>>,
    server: Option<ChunkDebugServer>,
    following: bool,
    port: u16
}
impl ChunkDebugTool {
    pub fn new() -> Self {
        Self {
            chunk_debug_map: Rc::new(RefCell::new(ChunkDebugMap::new())),
            server: None,
            following: false,
            port: 20000
        }
    }

    fn load_dump(&mut self, dump_path: String) -> Result<()> {
        let mut chunk_debug_map = self.chunk_debug_map.borrow_mut();
        chunk_debug_map.clear();

        let file = File::open(dump_path)?;
        for line in BufReader::new(file).lines() {
            let line = line?;

            let entry: ChunkDebugEntry = line.parse()?;
            chunk_debug_map.add_entry(entry);
        }

        chunk_debug_map.set_current_dimension(Dimension::Overworld);
        let available_ticks = chunk_debug_map.available_ticks();
        let first_tick = *available_ticks.first().unwrap_or(&0);
        chunk_debug_map.set_current_tick(first_tick);
        Ok(())
    }

    fn start_server(&mut self, task_list: &mut TaskList<State>, port: u16) {
        if self.server.is_some() {
            println!("Server Already Running");
            return;
        }

        let mut chunk_debug_map = self.chunk_debug_map.borrow_mut();
        chunk_debug_map.clear();

        let server = ChunkDebugServer::start(port, task_list);
        self.server = Some(server);
        self.following = true;
    }

    fn poll(&mut self) -> Result<()> {
        let mut done = false;
        if let Some(server) = &mut self.server {
            let mut received = false;
            for entry in server.poll() {
                self.chunk_debug_map.borrow_mut().add_entry(entry);
                received = true;
            }
            if self.following && received {
                self.chunk_debug_map.borrow_mut().step_end();
            }
            done = server.get_status() == ServerStatus::Stopped;
        }

        if done {
            self.server = None;
        }

        Ok(())
    }
}
impl Tool for ChunkDebugTool {
    fn start(&mut self, state: &mut CommonState) {
        let mut layer_group = LayerGroup::new();
        let provider = CheckerboardProvider::new(Rc::clone(&self.chunk_debug_map), Color::from_rgb(20, 20, 20));
        let layer = VirtualChunkLayer::new(provider);
        layer_group.add_layer("debug", layer, 0);
        state.layers.add_layer("chunk_debug", layer_group, 0);
    }

    fn stop(&mut self, state: &mut CommonState) {
        state.layers.remove_layer("chunk_debug");
    }

    fn gui(&mut self, state: &mut CommonState, task_list: &mut TaskList<State>, gui_ctx: &GuiContext) {
        self.poll().unwrap();
        egui::Window::new("1.8 Chunk Debug").show(gui_ctx, |ui| {
            egui::Grid::new("chunk_debug_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {

                    if ui.add_enabled(!self.server.is_some(), egui::Button::new("Open Dump")).clicked() {
                        let dump_path = tinyfiledialogs::open_file_dialog("Open", "", Some((&["*.csv"], ".csv")));
                        if let Some(dump_path) = dump_path {
                            self.load_dump(dump_path).unwrap();
                        }
                    }
                    ui.end_row();

                    if let Some(server) = &mut self.server {
                        if ui.button("Stop Server").clicked() {
                            match server.shutdown() {
                                Ok(_) => {}
                                Err(err) => println!("Error shutting down server {err}")
                            }
                        }

                        if let ServerStatus::Connected(addr) = server.get_status() {
                            ui.label(format!("Connected {addr}"));
                        } else {
                            ui.label(format!("Listening on {}", self.port));
                        }
                    } else {
                        if ui.button("Start Server").clicked() {
                            self.start_server(task_list, self.port);
                        }
                        ui.horizontal(|ui| {
                            ui.label("Port: ");
                            ui.add(egui::DragValue::new(&mut self.port).clamp_range(1024..=65535));
                        });
                    }
                    ui.end_row();

                    let mut current_dimension = self.chunk_debug_map.borrow().get_current_dimension();
                    ui.label("Current Dimension");
                    egui::ComboBox::new("current_dimension", "")
                        .selected_text(format!("{:?}", current_dimension))
                        .show_ui(ui, |ui| {
                            for dimension in [Dimension::Overworld, Dimension::Nether, Dimension::End] {
                                if ui.selectable_value(&mut current_dimension, dimension, format!("{:?}", dimension)).clicked() {
                                    self.chunk_debug_map.borrow_mut().set_current_dimension(dimension);
                                }
                            }
                        });
                    ui.end_row();

                    let mut current_tick = self.chunk_debug_map.borrow().get_current_tick();
                    ui.label("Current Tick");
                    egui::ComboBox::new("current_tick", "")
                        .selected_text(format!("{current_tick}"))
                        .show_ui(ui, |ui| {
                            let available_ticks = self.chunk_debug_map.borrow().available_ticks();
                            for tick in available_ticks {
                                if ui.selectable_value(&mut current_tick, tick, format!("{tick}")).clicked() {
                                    self.chunk_debug_map.borrow_mut().set_current_tick(tick);
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Controls");
                    ui.horizontal(|ui| {
                        if ui.button("|<").clicked() {
                            if let Some(chunk) = state.get_selected_chunk() {
                                self.chunk_debug_map.borrow_mut().step_home_chunk(chunk);
                            } else {
                                self.chunk_debug_map.borrow_mut().step_home();
                            }
                            self.following = false;
                        }
                        if ui.button("<").clicked() {
                            if let Some(chunk) = state.get_selected_chunk() {
                                self.chunk_debug_map.borrow_mut().step_back_chunk(chunk);
                            } else {
                                self.chunk_debug_map.borrow_mut().step_back();
                            }
                            self.following = false;
                        }
                        if ui.button(">").clicked() {
                            if let Some(chunk) = state.get_selected_chunk() {
                                self.chunk_debug_map.borrow_mut().step_forward_chunk(chunk);
                            } else {
                                self.chunk_debug_map.borrow_mut().step_forward();
                            }
                            self.following = false;
                        }
                        if ui.button(">|").clicked() {
                            if let Some(chunk) = state.get_selected_chunk() {
                                self.chunk_debug_map.borrow_mut().step_end_chunk(chunk);
                            } else {
                                self.chunk_debug_map.borrow_mut().step_end();
                            }
                            self.following = true;
                        }
                    });
                    ui.end_row();
                });

            if let Some(chunk) = state.get_selected_chunk() {
                if let Some(entries) = self.chunk_debug_map.borrow().entries_for_chunk(chunk) {
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
        });
    }
}