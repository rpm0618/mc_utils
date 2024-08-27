mod tool;
mod server;
mod map;

pub use tool::ChunkDebugTool;

use anyhow::{bail, Result};
use std::str::FromStr;
use ggez::graphics::Color;
use mc_utils::positions::ChunkPos;

use serde::Deserialize;
use mc_utils::world::Dimension;

use base64::prelude::*;

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
    tick: u32,
    dimension: Dimension,
    order: i32,
    event: Event,
    metadata: Metadata
}
impl FromStr for ChunkDebugEntry {
    type Err = anyhow::Error;

    fn from_str(line: &str) -> Result<Self> {
        let parts: Vec<_> = line.split(",").collect();
        let position = ChunkPos::new(parts[0].parse()?, parts[1].parse()?);
        let tick: u32 = parts[2].parse()?;
        let dimension = match parts[3] {
            "0" => Dimension::Overworld,
            "-1" => Dimension::Nether,
            "1" => Dimension::End,
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

        let entry = ChunkDebugEntry {
            position,
            tick,
            dimension,
            order: 0,
            event,
            metadata
        };
        Ok(entry)
    }
}