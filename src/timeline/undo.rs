//! Undo/Redo system for timeline editing

use crate::core::id::{TrackId, TrackItemId, ClipId, MarkerId, TransitionId, CommandId};
use crate::core::time::{Frame, FrameRange};
use crate::core::types::{EditOperation, TrimEdge, TrackType, EffectParams, EffectType, EffectValue};
use crate::core::error::Result;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

/// Undo command
pub trait Command: Send + Sync {
    fn execute(&mut self, timeline: &mut crate::timeline::clip::Timeline) -> Result<()>;
    fn undo(&mut self, timeline: &mut crate::timeline::clip::Timeline) -> Result<()>;
    fn description(&self) -> String;
    fn id(&self) -> CommandId;
    fn merges_with(&self, other: &dyn Command) -> bool { false }
    fn merge(&mut self, other: Box<dyn Command>) -> Result<()> { Ok(()) }
}

/// Command that stores before/after state
pub struct StateCommand {
    id: CommandId,
    description: String,
    before: Vec<Snapshot>,
    after: Vec<Snapshot>,
    executed: bool,
}

#[derive(Debug, Clone)]
pub enum Snapshot {
    ClipAdded { item_id: TrackItemId, data: ItemData },
    ClipRemoved { item_id: TrackItemId, data: ItemData },
    ClipMoved { item_id: TrackItemId, from: Frame, to: Frame },
    ClipTrimmed { item_id: TrackItemId, edge: TrimEdge, from: Frame, to: Frame },
    ClipSplit { item_id: TrackItemId, new_item_id: TrackItemId, position: Frame },
    ClipsJoined { item_id_a: TrackItemId, item_id_b: TrackItemId },
    EffectAdded { item_id: TrackItemId, effect: EffectData },
    EffectRemoved { item_id: TrackItemId, effect_index: usize },
    EffectModified { item_id: TrackItemId, effect_index: usize, params: EffectParams },
    TrackAdded { track_id: TrackId, track_type: TrackType, index: u32 },
    TrackRemoved { track_id: TrackId },
    TrackReordered { track_id: TrackId, from_index: u32, to_index: u32 },
    MarkerAdded { marker_id: MarkerId, position: Frame, data: MarkerData },
    MarkerRemoved { marker_id: MarkerId },
    MarkerModified { marker_id: MarkerId, data: MarkerData },
    TransitionAdded { transition_id: TransitionId, left: TrackItemId, right: TrackItemId, transition_type: EffectType, duration: Frame },
    TransitionRemoved { transition_id: TransitionId },
    ProjectSettingsChanged { old: crate::core::types::ProjectSettings, new: crate::core::types::ProjectSettings },
    TimelineSettingsChanged { old: crate::timeline::clip::TimelineSettings, new: crate::timeline::clip::TimelineSettings },
}

#[derive(Debug, Clone)]
pub struct ItemData {
    pub clip_id: Option<ClipId>,
    pub source_range: FrameRange,
    pub timeline_range: FrameRange,
    pub track_id: TrackId,
    pub track_index: u32,
    pub enabled: bool,
    pub locked: bool,
    pub opacity: f32,
    pub blend_mode: crate::timeline::track::BlendMode,
    pub volume: f32,
    pub pan: f32,
    pub speed: f32,
    pub effects: Vec<EffectData>,
    pub transitions: (Option<TransitionId>, Option<TransitionId>),
    pub color: crate::timeline::track::TrackItemColor,
    pub metadata: crate::timeline::track::TrackItemMetadata,
}

#[derive(Debug, Clone)]
pub struct EffectData {
    pub effect_type: EffectType,
    pub enabled: bool,
    pub parameters: std::collections::HashMap<String, EffectValue>,
    pub keyframes: std::collections::HashMap<String, Vec<crate::core::types::Keyframe>>,
}

#[derive(Debug, Clone)]
pub struct MarkerData {
    pub name: String,
    pub note: String,
    pub color: crate::core::types::ColorLabel,
    pub duration: Option<Frame>,
    pub marker_type: crate::core::types::MarkerType,
}

impl StateCommand {
    pub fn new(description: String) -> Self {
        Self {
            id: CommandId::generate(),
            description,
            before: Vec::new(),
            after: Vec::new(),
            executed: false,
        }
    }

    pub fn record_before(&mut self, snapshot: Snapshot) {
        if !self.executed {
            self.before.push(snapshot);
        }
    }

    pub fn record_after(&mut self, snapshot: Snapshot) {
        if self.executed {
            self.after.push(snapshot);
        }
    }

    pub fn set_executed(&mut self) { self.executed = true; }
}

impl Command for StateCommand {
    fn execute(&mut self, timeline: &mut crate::timeline::clip::Timeline) -> Result<()> {
        // Apply after state
        for snapshot in &self.after {
            apply_snapshot(timeline, snapshot)?;
        }
        self.executed = true;
        Ok(())
    }

    fn undo(&mut self, timeline: &mut crate::timeline::clip::Timeline) -> Result<()> {
        // Apply before state in reverse
        for snapshot in self.before.iter().rev() {
            apply_snapshot(timeline, snapshot)?;
        }
        self.executed = false;
        Ok(())
    }

    fn description(&self) -> String { self.description.clone() }
    fn id(&self) -> CommandId { self.id }
}

/// Command history manager
pub struct UndoManager {
    undo_stack: VecDeque<Box<dyn Command>>,
    redo_stack: VecDeque<Box<dyn Command>>,
    max_levels: usize,
    current_group: Option<CommandGroup>,
    timeline: Arc<RwLock<crate::timeline::clip::Timeline>>,
}

struct CommandGroup {
    description: String,
    commands: Vec<Box<dyn Command>>,
}

impl UndoManager {
    pub fn new(timeline: Arc<RwLock<crate::timeline::clip::Timeline>>, max_levels: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_levels,
            current_group: None,
            timeline,
        }
    }

    pub fn push(&mut self, mut command: Box<dyn Command>) -> Result<()> {
        // Execute command
        command.execute(&mut *self.timeline.write())?;
        
        // Check for merge
        if let Some(last) = self.undo_stack.back() {
            if last.merges_with(command.as_ref()) {
                last.merge(command)?;
                return Ok(());
            }
        }
        
        // Add to undo stack
        self.undo_stack.push_back(command);
        
        // Limit stack size
        while self.undo_stack.len() > self.max_levels {
            self.undo_stack.pop_front();
        }
        
        // Clear redo stack
        self.redo_stack.clear();
        
        Ok(())
    }

    pub fn undo(&mut self) -> Result<()> {
        if let Some(mut command) = self.undo_stack.pop_back() {
            command.undo(&mut *self.timeline.write())?;
            self.redo_stack.push_back(command);
            Ok(())
        } else {
            Err(crate::core::error::TimelineError::UndoEmpty.into())
        }
    }

    pub fn redo(&mut self) -> Result<()> {
        if let Some(mut command) = self.redo_stack.pop_back() {
            command.execute(&mut *self.timeline.write())?;
            self.undo_stack.push_back(command);
            Ok(())
        } else {
            Err(crate::core::error::TimelineError::RedoEmpty.into())
        }
    }

    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }

    pub fn undo_description(&self) -> Option<String> {
        self.undo_stack.back().map(|c| c.description())
    }

    pub fn redo_description(&self) -> Option<String> {
        self.redo_stack.back().map(|c| c.description())
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn start_group(&mut self, description: String) {
        self.current_group = Some(CommandGroup {
            description,
            commands: Vec::new(),
        });
    }

    pub fn end_group(&mut self) -> Result<()> {
        if let Some(group) = self.current_group.take() {
            if group.commands.is_empty() { return Ok(()); }
            
            // Merge into single command
            let mut merged = StateCommand::new(group.description);
            for cmd in group.commands {
                // Extract snapshots from each command
            }
            self.push(Box::new(merged))?;
        }
        Ok(())
    }

    pub fn add_to_group(&mut self, command: Box<dyn Command>) -> Result<()> {
        if let Some(group) = &mut self.current_group {
            command.execute(&mut *self.timeline.write())?;
            group.commands.push(command);
        } else {
            self.push(command)?;
        }
        Ok(())
    }
}

fn apply_snapshot(
    timeline: &mut crate::timeline::clip::Timeline,
    snapshot: &Snapshot,
) -> Result<()> {
    use crate::timeline::track::TrackItemsMut;
    
    match snapshot {
        Snapshot::ClipAdded { item_id, data } => {
            let mut item = TrackItem {
                id: *item_id,
                key: TrackItemKey::default(),
                track_id: data.track_id,
                clip_id: data.clip_id,
                source_range: data.source_range,
                timeline_range: data.timeline_range,
                track_index: data.track_index,
                enabled: data.enabled,
                locked: data.locked,
                opacity: data.opacity,
                blend_mode: data.blend_mode,
                volume: data.volume,
                pan: data.pan,
                speed: data.speed,
                effects: data.effects.clone(),
                transitions: data.transitions,
                color: data.color,
                metadata: data.metadata.clone(),
            };
            item.key = TrackItemKey::default();
            
            if let Some(track) = timeline.get_track_mut(data.track_id) {
                track.items_mut().insert(data.timeline_range.start, item.key);
            }
            timeline.items.insert(*item_id, item);
        }
        
        Snapshot::ClipRemoved { item_id, .. } => {
            if let Some(item) = timeline.items.remove(item_id) {
                if let Some(track) = timeline.get_track_mut(item.track_id) {
                    track.items_mut().remove(&item.timeline_range.start);
                }
            }
        }
        
        Snapshot::ClipMoved { item_id, to, .. } => {
            if let Some(item) = timeline.get_item_mut(*item_id) {
                let duration = item.timeline_duration();
                item.timeline_range = FrameRange::new(*to, *to + duration);
                if let Some(track) = timeline.get_track_mut(item.track_id) {
                    track.items_mut().remove(&item.timeline_range.start);
                    track.items_mut().insert(*to, item.key);
                }
            }
        }
        
        Snapshot::ClipTrimmed { item_id, edge, to, .. } => {
            if let Some(item) = timeline.get_item_mut(*item_id) {
                match edge {
                    TrimEdge::In => {
                        let delta = *to - item.timeline_in();
                        item.trim_in(delta);
                    }
                    TrimEdge::Out => {
                        let delta = *to - item.timeline_out();
                        item.trim_out(delta);
                    }
                }
            }
        }
        
        Snapshot::EffectAdded { item_id, effect } => {
            if let Some(item) = timeline.get_item_mut(*item_id) {
                item.effects.push(crate::timeline::clip::TrackItemEffect {
                    effect_type: effect.effect_type,
                    enabled: effect.enabled,
                    parameters: effect.parameters.clone(),
                    keyframes: effect.keyframes.clone(),
                });
            }
        }
        
        Snapshot::EffectRemoved { item_id, effect_index } => {
            if let Some(item) = timeline.get_item_mut(*item_id) {
                if *effect_index < item.effects.len() {
                    item.effects.remove(*effect_index);
                }
            }
        }
        
        Snapshot::EffectModified { item_id, effect_index, params } => {
            if let Some(item) = timeline.get_item_mut(*item_id) {
                if *effect_index < item.effects.len() {
                    item.effects[*effect_index].parameters = params.parameters.clone();
                    item.effects[*effect_index].keyframes = params.keyframes.clone();
                }
            }
        }
        
        Snapshot::MarkerAdded { marker_id, position, data } => {
            timeline.markers.insert(*marker_id, crate::timeline::clip::Marker {
                id: *marker_id,
                position: *position,
                duration: data.duration,
                name: data.name.clone(),
                note: data.note.clone(),
                color: data.color,
                marker_type: data.marker_type,
            });
        }
        
        Snapshot::MarkerRemoved { marker_id } => {
            timeline.markers.remove(marker_id);
        }
        
        Snapshot::MarkerModified { marker_id, data } => {
            if let Some(marker) = timeline.markers.get_mut(marker_id) {
                marker.name = data.name.clone();
                marker.note = data.note.clone();
                marker.color = data.color;
                marker.duration = data.duration;
                marker.marker_type = data.marker_type;
            }
        }
        
        _ => {}
    }
    
    timeline.update_duration();
    timeline.dirty = true;
    Ok(())
}

/// Macro for creating commands easily
macro_rules! command {
    ($desc:expr, $timeline:expr, |$tl:ident| $body:expr) => {{
        let mut cmd = StateCommand::new($desc.to_string());
        let mut tl_guard = $timeline.write();
        // Record before state
        $body(&mut tl_guard, &mut cmd);
        cmd.set_executed();
        // Record after state would need another pass
        Box::new(cmd) as Box<dyn Command>
    }};
}