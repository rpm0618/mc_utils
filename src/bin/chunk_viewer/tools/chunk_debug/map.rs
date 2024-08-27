use std::collections::{HashMap, HashSet};
use ggez::graphics::Color;
use mc_utils::positions::ChunkPos;
use mc_utils::world::Dimension;
use crate::chunk_viewer::chunk_layer::{ChunkColor, VirtualChunkProvider};
use crate::chunk_viewer::tools::chunk_debug::{ChunkDebugEntry, Event};
use crate::chunk_viewer::viewport::Viewport;

#[derive(Debug, Clone)]
// Holds all entries for a single tick (in one dimension)
// Entries are stored in a map by chunk position, in a list in time order within the tick
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

// All entry holders for all ticks for a single dimension.
// Entry holders are stored ordered by tick in a list
// An index maps chunk positions to a list of indices into the entry holders list
struct EntryMap {
    entry_holders: Vec<EntryHolder>,
    chunk_entry_holders: HashMap<ChunkPos, Vec<usize>>
}
impl EntryMap {
    fn new() -> Self {
        Self {
            entry_holders: Vec::new(),
            chunk_entry_holders: HashMap::new()
        }
    }

    fn add_entry(&mut self, mut entry: ChunkDebugEntry) {
        let tick = entry.tick;

        if self.entry_holders.is_empty() {
            self.entry_holders.push(EntryHolder::new(tick));
        }
        let mut entry_holder_index = self.entry_holders.len() - 1;
        let mut entry_holder = self.entry_holders.last_mut().unwrap();

        if entry_holder.tick > tick {
            panic!("Entry added out of order!");
        }

        if entry_holder.tick < tick {
            self.entry_holders.push(EntryHolder::new(tick));
            entry_holder = self.entry_holders.last_mut().unwrap();
            entry_holder_index += 1;
        }

        if !self.chunk_entry_holders.contains_key(&entry.position) {
            self.chunk_entry_holders.insert(entry.position, Vec::new());
        }
        let chunk_entry_holders = self.chunk_entry_holders.get_mut(&entry.position).unwrap();
        if chunk_entry_holders.last().is_none_or(|eh_index| *eh_index != entry_holder_index) {
            chunk_entry_holders.push(entry_holder_index);
        }

        entry.order = entry_holder.total_size as i32;
        entry_holder.add_entry(entry);
    }
}

// Holds the full status of the chunk debug map (EntryMaps for each dimension, the current dimension
// and tick). max_entry_holder_index is a cached value, indicating the entry_holder corresponding to
// the current tick. Also acts as a VirtualChunkProvder
pub(crate) struct ChunkDebugMap {
    entry_maps: HashMap<Dimension, EntryMap>,
    current_dimension: Dimension,
    current_tick: u32,
    max_entry_holder_index: usize,
    dirty: bool
}
impl ChunkDebugMap {
    pub fn new() -> Self {
        let mut entry_maps = HashMap::new();
        entry_maps.insert(Dimension::Overworld, EntryMap::new());
        entry_maps.insert(Dimension::Nether, EntryMap::new());
        entry_maps.insert(Dimension::End, EntryMap::new());

        Self {
            entry_maps,
            current_dimension: Dimension::Overworld,
            current_tick: 0,
            max_entry_holder_index: 0,
            dirty: false
        }
    }

    pub fn add_entry(&mut self, entry: ChunkDebugEntry) {
        if entry.dimension == self.current_dimension {
            self.dirty = true;
        }
        let entry_map = self.entry_maps.get_mut(&entry.dimension).unwrap();
        entry_map.add_entry(entry);
    }

    pub fn available_ticks(&self) -> Vec<u32> {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        entry_map.entry_holders.iter().map(|eh| eh.tick).collect()
    }

    pub fn get_current_tick(&self) -> u32 {
        self.current_tick
    }    
    
    pub fn set_current_tick(&mut self, tick: u32) {
        self.current_tick = tick;
        self.dirty = true;
        self.recalc_max_entry_holder_index();
    }

    // Step to the first tick that has an event
    pub fn step_home(&mut self) {
        self.max_entry_holder_index = 0;
        self.dirty = true;
        self.recalc_current_tick();
    }

    // Step to the first tick that has an event for this chunk
    pub fn step_home_chunk(&mut self, chunk: ChunkPos) {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        let eh_indices = entry_map.chunk_entry_holders.get(&chunk);
        if let Some(eh_indices) = eh_indices {
            self.max_entry_holder_index = eh_indices[0];
            self.dirty = true;
            self.recalc_current_tick()
        }
    }

    // Step back to the previous tick (that we have an event for)
    pub fn step_back(&mut self) {
        if self.max_entry_holder_index > 0 {
            self.max_entry_holder_index -= 1;
            self.dirty = true;
            self.recalc_current_tick();
        }
    }

    // Step back to the previous tick that has an event in given chunk
    pub fn step_back_chunk(&mut self, chunk: ChunkPos) {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        let eh_indices = entry_map.chunk_entry_holders.get(&chunk);
        if let Some(eh_indices) = eh_indices {
            // Two cases:
            // - Ok(index): Chunk has an entry holder for the current tick. index is the location of
            // that entry holder
            // - Err(index): Chunk has no events on the current tick. index is the location of the
            // entry holder where it should be inserted to maintain sorted order.
            //
            // Either way, we want to subtract 1 from the index to find the previous chunk
            let index = eh_indices.binary_search(&self.max_entry_holder_index).unwrap_or_else(|next_index| next_index);
            if index > 0 {
                self.max_entry_holder_index = eh_indices[index - 1];
                self.dirty = true;
                self.recalc_current_tick();
            }
        }
    }

    // Step forward to the next tick (that we have an event for)
    pub fn step_forward(&mut self) {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        let entry_holders = &entry_map.entry_holders;
        if self.max_entry_holder_index < entry_holders.len() - 1 {
            self.max_entry_holder_index += 1;
            self.dirty = true;
            self.recalc_current_tick();
        }
    }

    // Step forward to the next tick with an event in given chunk
    pub fn step_forward_chunk(&mut self, chunk: ChunkPos) {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        let eh_indices = entry_map.chunk_entry_holders.get(&chunk);
        if let Some(eh_indices) = eh_indices {
            // Two cases:
            // - Ok(index): Chunk has an entry holder for the current tick. index is the location of
            // that entry holder
            // - Err(index): Chunk has no events on the current tick. index is the location of the
            // entry holder where it should be inserted to maintain sorted order.
            //
            // In the first case we need to increment the found index to step forward, for the
            // latter this isn't needed.
            let next_index = match eh_indices.binary_search(&self.max_entry_holder_index) {
                Ok(index) => index + 1,
                Err(next_index) => next_index
            };
            if next_index < eh_indices.len() {
                self.max_entry_holder_index = eh_indices[next_index];
                self.dirty = true;
                self.recalc_current_tick();
            }
        }
    }

    // Step to the last tick that has an event
    pub fn step_end(&mut self) {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        let entry_holders = &entry_map.entry_holders;
        if entry_holders.len() > 0 {
            self.max_entry_holder_index = entry_holders.len() - 1;
            self.dirty = true;
            self.recalc_current_tick();
        }
    }

    // Step to the last tick that has an event for this chunk
    pub fn step_end_chunk(&mut self, chunk: ChunkPos) {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        let eh_indices = entry_map.chunk_entry_holders.get(&chunk);
        if let Some(eh_indices) = eh_indices {
            self.max_entry_holder_index = *eh_indices.last().unwrap();
            self.dirty = true;
            self.recalc_current_tick()
        }
    }

    pub fn get_current_dimension(&self) -> Dimension {
        self.current_dimension
    }
    
    pub fn set_current_dimension(&mut self, dimension: Dimension) {
        self.current_dimension = dimension;
        self.dirty = true;
        self.recalc_max_entry_holder_index();
    }

    pub fn entries_for_chunk(&self, position: ChunkPos) -> Option<&Vec<ChunkDebugEntry>> {
        let entry_map = self.entry_maps.get(&self.current_dimension)?;
        let entry_holder = &entry_map.entry_holders.get(self.max_entry_holder_index)?;
        entry_holder.entries.get(&position)
    }

    fn recalc_current_tick(&mut self) {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        self.current_tick = entry_map.entry_holders.get(self.max_entry_holder_index)
            .map(|eh| eh.tick)
            .unwrap_or(0);
    }

    fn recalc_max_entry_holder_index(&mut self) {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        self.max_entry_holder_index = entry_map.entry_holders.binary_search_by_key(&self.current_tick, |eh| eh.tick).unwrap_or_else(|i| i);
    }

    pub fn clear(&mut self) {
        self.entry_maps.clear();
        self.entry_maps.insert(Dimension::Overworld, EntryMap::new());
        self.entry_maps.insert(Dimension::Nether, EntryMap::new());
        self.entry_maps.insert(Dimension::End, EntryMap::new());
        self.dirty = true;
    }
}
impl VirtualChunkProvider for ChunkDebugMap {
    fn chunks_in_viewport(&self, _viewport: &Viewport, already_rendered: &HashSet<ChunkPos>) -> Vec<(ChunkPos, ChunkColor)> {
        let entry_map = self.entry_maps.get(&self.current_dimension).unwrap();
        let result = entry_map.chunk_entry_holders.iter().filter_map(move |(position, chunk_eh_indices)| {
            if already_rendered.contains(position) {
                return None;
            }
            match chunk_eh_indices.binary_search(&self.max_entry_holder_index) {
                Ok(chunk_eh_index) => {
                    // The chunk has events on the current tick
                    let entry_holder = &entry_map.entry_holders[chunk_eh_indices[chunk_eh_index]];
                    let entries = entry_holder.entries.get(position).expect("Corrupted EntryMap");
                    let index = entries.len() - 1;
                    let primary = entries[index].event.color();
                    let secondary = if index > 0 {
                        Some(entries[index - 1].event.color())
                    } else {
                        None
                    };
                    let chunk_color = ChunkColor::new(primary, secondary);
                    return Some((*position, chunk_color));
                }
                Err(next_eh_index) => {
                    // This chunk doesn't have any events on the current tick, so look at the
                    // previous event to determine what to render
                    if next_eh_index == 0 {
                        return Some((*position, ChunkColor::new(Color::from_rgb(50, 50, 50), None)));
                    }
                    let prev_eh_index = next_eh_index - 1;
                    let prev_entry_holder = &entry_map.entry_holders[chunk_eh_indices[prev_eh_index]];
                    let prev_entries = prev_entry_holder.entries.get(position).expect("Corrupted EntryMap");
                    let last = prev_entries.last().unwrap();
                    if last.event != Event::Unloaded {
                        let event = if last.event == Event::UnloadScheduled {
                            Event::UnloadScheduled
                        } else {
                            Event::AlreadyLoaded
                        };
                        return Some((*position, ChunkColor::new(event.color(), None)));
                    }
                }
            }

            Some((*position, ChunkColor::new(Color::from_rgb(50, 50, 50), None)))
        }).collect();

        result
    }

    fn check_dirty(&mut self) -> bool {
        let result = self.dirty;
        self.dirty = false;
        result
    }
}