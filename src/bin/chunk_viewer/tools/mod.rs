pub mod nether_falling_block;
pub mod chunk_debug;

use std::any::Any;
use ggegui::GuiContext;
use std::collections::HashMap;
use mc_utils::positions::ChunkPos;
use crate::chunk_viewer::event_handler::{CommonState, State};
use crate::chunk_viewer::task_list::TaskList;

pub trait Tool: Any {
    fn start(&mut self, state: &mut CommonState);
    fn stop(&mut self, state: &mut CommonState);
    fn gui(&mut self, state: &mut CommonState, task_list: &mut TaskList<State>, gui_ctx: &GuiContext);

    fn on_chunk_selected(&mut self, _chunk: ChunkPos, _state: &mut CommonState) {}
}

pub struct Toolbox {
    tools: HashMap<String, Box<dyn Tool>>,
    current_tool: Option<String>
}

impl Toolbox {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            current_tool: None
        }
    }

    pub fn add_tool(&mut self, name: &str, tool: impl Tool + 'static) {
        self.tools.insert(name.to_string(), Box::new(tool));
    }

    pub fn set_current_tool(&mut self, name: &str, state: &mut CommonState) {
        if !self.tools.contains_key(name) {
            panic!("Unknown tool {name}");
        }
        if let Some(current_tool) = &self.current_tool {
            self.tools.get_mut(current_tool).unwrap().stop(state);
        }
        self.current_tool = Some(name.to_string());
        self.tools.get_mut(self.current_tool.as_ref().unwrap()).unwrap().start(state);
    }

    pub fn get_current_tool<T: Tool + 'static>(&self) -> Option<&T> {
        self.get_tool(self.current_tool.as_ref()?)
    }

    pub fn get_current_tool_mut<T: Tool + 'static>(&mut self) -> Option<&mut T> {
        let current_tool = self.current_tool.as_ref()?.clone();
        self.get_tool_mut(&current_tool)
    }

    pub fn get_tool_names(&self) -> impl Iterator<Item=&String> {
        self.tools.keys()
    }
    
    pub fn get_current_tool_name(&self) -> Option<&String> {
        self.current_tool.as_ref()
    }

    pub fn get_tool<T: Tool + 'static>(&self, name: &str) -> Option<&T> {
        let tool = self.tools.get(name)?;
        let tool = tool.as_ref() as &dyn Any;
        Some(tool.downcast_ref::<T>()?)
    }

    pub fn get_tool_mut<T: Tool + 'static>(&mut self, name: &str) -> Option<&mut T> {
        let tool = self.tools.get_mut(name)?;
        let tool = tool.as_mut() as &mut dyn Any;
        Some(tool.downcast_mut::<T>()?)
    }

    pub fn gui(&mut self, state: &mut CommonState, task_list: &mut TaskList<State>, gui_ctx: &GuiContext) {
        if let Some(current_tool) = &self.current_tool {
            self.tools.get_mut(current_tool).unwrap().gui(state, task_list, gui_ctx);
        }
    }

    pub fn on_chunk_selected(&mut self, chunk: ChunkPos, state: &mut CommonState) {
        if let Some(current_tool) = self.current_tool.as_ref() {
            self.tools.get_mut(current_tool).unwrap().on_chunk_selected(chunk, state);
        }
    }
}