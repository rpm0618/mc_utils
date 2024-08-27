use std::cmp::{max, min};
use std::collections::HashSet;
use ggegui::{egui, Gui};
use ggegui::egui::Align2;
use ggez::{Context, event, GameError, GameResult, graphics};
use ggez::event::MouseButton;
use ggez::graphics::{Color, DrawMode, DrawParam, Mesh};
use mc_utils::positions::ChunkPos;
use crate::chunk_viewer::viewport::Viewport;

use crate::chunk_viewer::chunk_layer::{ChunkLayer, DiagonalProvider, HashSetLayer, LayerGroup, VirtualChunkLayer};
use crate::chunk_viewer::task_list::TaskList;
use crate::chunk_viewer::tools::nether_falling_block::NetherFallingBlockTool;
use crate::chunk_viewer::tools::{Toolbox};
use crate::chunk_viewer::tools::chunk_debug::ChunkDebugTool;

#[derive(PartialEq, Eq, Copy, Clone)]
enum SelectionMode {
    Single,
    Add,
    Subtract
}

pub struct CommonState {
    pub viewport: Viewport,
    pub layers: LayerGroup,

    pub selection: HashSet<ChunkPos>,

    selection_rect_origin: Option<ChunkPos>,
    
    dragging: bool,
    selecting_range: bool,
    world_diagonals: bool,
    selection_mode: SelectionMode
}
impl CommonState {
    fn on_range_selection(&mut self, corner1: ChunkPos, corner2: ChunkPos) {
        let top_left = ChunkPos::new(min(corner1.x, corner2.x), min(corner1.z, corner2.z));
        let bottom_right = ChunkPos::new(max(corner1.x, corner2.x), max(corner1.z, corner2.z));
        if self.selection_mode == SelectionMode::Single {
            self.selection.clear();
        }
        for x in top_left.x..=bottom_right.x {
            for z in top_left.z..=bottom_right.z {
                let chunk = ChunkPos::new(x, z);
                match self.selection_mode {
                    SelectionMode::Add | SelectionMode::Single => {self.selection.insert(chunk);}
                    SelectionMode::Subtract => {self.selection.remove(&chunk);}
                }
            }
        }

        self.refresh_selection_layer();
    }

    fn on_single_selection(&mut self, chunk: ChunkPos) {
        match self.selection_mode {
            SelectionMode::Single => {
                self.selection.clear();
                self.selection.insert(chunk);
            }
            SelectionMode::Add => {self.selection.insert(chunk);}
            SelectionMode::Subtract => {self.selection.remove(&chunk);}
        }

        self.refresh_selection_layer();
    }

    fn refresh_selection_layer(&mut self) {
        let selection_layer = self.layers.get_layer_mut::<HashSetLayer>("selection").unwrap();
        selection_layer.set_chunks(self.selection.clone());
    }

    pub fn get_selected_chunk(&self) -> Option<ChunkPos> {
        if self.selection.len() == 1 {
            Some(*self.selection.iter().next()?)
        } else {
            None
        }
    }
}

pub struct State {
    pub common_state: CommonState,
    pub toolbox: Toolbox
}

pub struct ViewerEventHandler {
    gui: Gui,
    state: State,
    task_list: TaskList<State>,
}

impl ViewerEventHandler {
    pub fn new(ctx: &mut Context) -> GameResult<ViewerEventHandler> {
        let mut layer_group = LayerGroup::new();
        layer_group.add_layer("diagonals", VirtualChunkLayer::new(DiagonalProvider::new(Color::RED)), 5);
        layer_group.set_visible("diagonals", false);
        layer_group.add_layer("selection", HashSetLayer::new(HashSet::new(), Color::from_rgba(0, 0, 0, 128)), 10);

        let mut toolbox = Toolbox::new();
        toolbox.add_tool("Nether Falling Block", NetherFallingBlockTool::new());
        toolbox.add_tool("1.8 Chunk Debug", ChunkDebugTool::new());

        let mut result = ViewerEventHandler {
            gui: Gui::new(ctx),
            task_list: TaskList::new(),
            state: State {
                common_state: CommonState {
                    viewport: Viewport::new(),
                    layers: layer_group,
                    dragging: false,
                    selection: HashSet::new(),
                    selection_rect_origin: None,
                    selecting_range: false,
                    world_diagonals: false,
                    selection_mode: SelectionMode::Single
                },
                toolbox
            },
        };

        // result.state.toolbox.set_current_tool("Nether Falling Block", &mut result.state.common_state);
        result.state.toolbox.set_current_tool("1.8 Chunk Debug", &mut result.state.common_state);

        Ok(result)
    }
}

impl event::EventHandler<GameError> for ViewerEventHandler {
    fn update(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        let gui_ctx = self.gui.ctx();
        let state = &mut self.state.common_state;
        egui::Window::new("General").movable(false).show(&gui_ctx, |ui| {
            egui::Grid::new("general_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Chunk");
                    ui.horizontal(|ui| {
                        let mut mouse_chunk = state.viewport.chunk_at(ctx.mouse.position());
                        ui.add_enabled(false, egui::DragValue::new(&mut mouse_chunk.x));
                        ui.add_enabled(false, egui::DragValue::new(&mut mouse_chunk.z));
                    });
                    ui.end_row();

                    ui.label("Selection Mode");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut state.selection_mode, SelectionMode::Single, "Single");
                        ui.selectable_value(&mut state.selection_mode, SelectionMode::Add, "Add");
                        ui.selectable_value(&mut state.selection_mode, SelectionMode::Subtract, "Subtract");
                    });
                    ui.end_row();

                    if ui.button("Clear Selection").clicked() {
                        state.selection.clear();
                        state.refresh_selection_layer();
                    }
                    ui.label(format!("Selected Chunks {}", state.selection.len()));
                    ui.end_row();

                    ui.label("World Diagonals");
                    if ui.checkbox(&mut state.world_diagonals, "").changed() {
                        state.layers.set_visible("diagonals", state.world_diagonals);
                    }
                    ui.end_row();

                    ui.label("Tool");
                    let mut dummy_tool_name = String::new();
                    egui::ComboBox::new("current_tool", "")
                        .selected_text(self.state.toolbox.get_current_tool_name().unwrap_or(&"".to_string()))
                        .show_ui(ui, |ui| {
                            let mut names: Vec<_> = self.state.toolbox.get_tool_names().map(|t| t.clone()).collect();
                            names.sort();
                            for tool_name in &names {
                                if ui.selectable_value(&mut dummy_tool_name, tool_name.clone(), tool_name).clicked() {
                                    self.state.toolbox.set_current_tool(&tool_name, state);
                                }
                            }
                        })
                });
        });

        self.state.toolbox.gui(state, &mut self.task_list, &gui_ctx);

        if self.task_list.len() > 0 {
            egui::Window::new("Tasks").anchor(Align2::RIGHT_BOTTOM, [0.0, 0.0]).show(&gui_ctx, |ui| {
                egui::Grid::new("task_grid")
                    .num_columns(2)
                    .striped(true)
                    .show(ui, |ui| {
                        for (name, task) in &self.task_list {
                            let progress = task.progress();
                            ui.label(name);
                            let progress_bar = if progress > 0.0 {
                                egui::ProgressBar::new(progress).show_percentage()
                            } else {
                                egui::ProgressBar::new(progress).animate(true)
                            };
                            ui.add(progress_bar);
                            ui.end_row();
                        }
                    });
            });
        }
        self.gui.update(ctx);

        self.task_list.poll(&mut self.state);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> Result<(), GameError> {
        let state = &mut self.state.common_state;
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::from([0.392, 0.584, 0.929, 1.0]));
        let original_screen_coords = canvas.screen_coordinates();
        let screen_coords = state.viewport.get_screen_coordinates();
        if original_screen_coords.is_some() {
            canvas.set_screen_coordinates(screen_coords);
        }

        state.layers.render(&state.viewport, ctx, &mut canvas);

        let mouse_chunk = state.viewport.chunk_at(ctx.mouse.position());
        let chunk_rect = if let Some(origin) = state.selection_rect_origin {
            let top_left = ChunkPos::new(min(mouse_chunk.x, origin.x), min(mouse_chunk.z, origin.z));
            let bottom_right = ChunkPos::new(max(mouse_chunk.x, origin.x), max(mouse_chunk.z, origin.z));
            state.viewport.chunk_rect_to_rect(top_left, bottom_right)
        } else {
            state.viewport.chunk_to_rect(mouse_chunk)
        };
        let chunk_highlight = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            chunk_rect,
            Color::from_rgba(0, 0, 0, 128)
        )?;
        canvas.draw(&chunk_highlight, [0.0, 0.0]);

        if original_screen_coords.is_some() {
            canvas.set_screen_coordinates(original_screen_coords.unwrap());
        }
        canvas.draw(&self.gui, DrawParam::default());

        canvas.finish(ctx)?;

        Ok(())
    }

    fn mouse_button_down_event(&mut self, ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) -> Result<(), GameError> {
        let state = &mut self.state.common_state;
        if !self.gui.ctx().is_pointer_over_area() {
            if button == MouseButton::Left {
                state.dragging = true
            }
            if button == MouseButton::Right {
                let mouse_chunk = state.viewport.chunk_at(ctx.mouse.position());
                state.selection_rect_origin = Some(mouse_chunk);
            }
        }
        Ok(())
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) -> Result<(), GameError> {
        let state = &mut self.state.common_state;
        if state.dragging && button == MouseButton::Left {
            state.dragging = false;
        }
        if button == MouseButton::Right {
            let mouse_chunk = state.viewport.chunk_at(ctx.mouse.position());

            if state.selecting_range {
                state.selecting_range = false;
                state.on_range_selection(state.selection_rect_origin.unwrap(), mouse_chunk);
            } else {
                state.on_single_selection(mouse_chunk);
                self.state.toolbox.on_chunk_selected(mouse_chunk, state);
            }
            state.selection_rect_origin = None;
        }
        Ok(())
    }


    fn mouse_motion_event(&mut self, ctx: &mut Context, _x: f32, _y: f32, dx: f32, dy: f32) -> Result<(), GameError> {
        let state = &mut self.state.common_state;
        if state.dragging {
            state.viewport.translate([dx, dy]);
        }
        if let Some(origin) = state.selection_rect_origin {
            let mouse_chunk = state.viewport.chunk_at(ctx.mouse.position());
            if mouse_chunk != origin {
                state.selecting_range = true;
            }
        }
        Ok(())
    }

    fn mouse_wheel_event(&mut self, ctx: &mut Context, x: f32, y: f32) -> Result<(), GameError> {
        let state = &mut self.state.common_state;
        if !self.gui.ctx().wants_pointer_input() {
            state.viewport.zoom_into(y * 0.1, ctx.mouse.position());
        } else {
            self.gui.input.mouse_wheel_event(x, y * 10.0);
        }
        Ok(())
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char) -> Result<(), GameError> {
        self.gui.input.text_input_event(character);
        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) -> Result<(), GameError> {
        let state = &mut self.state.common_state;
        state.viewport.on_resize(width, height, ctx.gfx.window().scale_factor() as f32);
        self.gui.input.set_scale_factor(state.viewport.scale_factor * state.viewport.zoom, (width, height));
        Ok(())
    }
}