#![feature(trait_upcasting)]
mod chunk_viewer;

use ggez::{ContextBuilder, event,GameResult};
use ggez::conf::{WindowMode, WindowSetup};
use ggez::winit::dpi::LogicalSize;
use crate::chunk_viewer::event_handler::ViewerEventHandler;

fn main() -> GameResult {
    let (mut ctx, event_loop) = ContextBuilder::new("viewer", "rpm0618")
        .window_setup(WindowSetup {
            title: "Minecraft Chunk Viewer".to_string(),
            vsync: true,
            ..WindowSetup::default()
        })
        .window_mode(WindowMode {
            logical_size: Some(LogicalSize::new(800.0, 600.0)),
            resizable: true,
            ..WindowMode::default()
        })
        .build()?;
    let state = ViewerEventHandler::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
}