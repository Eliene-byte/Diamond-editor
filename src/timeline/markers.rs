//! Timeline markers - chapters, notes, sync points, etc.

use crate::core::id::MarkerId;
use crate::core::time::{Frame, FrameRange, Timecode, FrameRate};
use crate::core::types::{ColorLabel, MarkerType};
use std::collections::BTreeMap;

/// Marker manager
pub struct MarkerManager {
    markers: BTreeMap<Frame, Vec<Marker>>,
    by_id: std::collections::HashMap<MarkerId, Marker>,
    next_chapter: u32,
}

#[derive(Debug, Clone)]
pub struct Marker {
    pub id: MarkerId,
    pub position: Frame,
    pub duration: Option<Frame>,
    pub name: String,
    pub note: String,
    pub color: ColorLabel,
    pub marker_type: MarkerType,
    pub chapter_number: Option<u32>,
}

impl MarkerManager {
    pub fn new() -> Self {
        Self {
            markers: BTreeMap::new(),
            by_id: std::collections::HashMap::new(),
            next_chapter: 1,
        }
    }

    pub fn add(&mut self, mut marker: Marker) -> MarkerId {
        if marker.id.0 == 0 {
            marker.id = MarkerId::generate();
        }
        
        if marker.marker_type == MarkerType::Chapter && marker.chapter_number.is_none() {
            marker.chapter_number = Some(self.next_chapter);
            self.next_chapter += 1;
        }
        
        self.markers.entry(marker.position).or_default().push(marker.clone());
        self.by_id.insert(marker.id, marker.clone());
        marker.id
    }

    pub fn remove(&mut self, marker_id: MarkerId) -> Option<Marker> {
        if let Some(marker) = self.by_id.remove(&marker_id) {
            if let Some(vec) = self.markers.get_mut(&marker.position) {
                vec.retain(|m| m.id != marker_id);
                if vec.is_empty() {
                    self.markers.remove(&marker.position);
                }
            }
            
            // Renumber chapters
            if marker.marker_type == MarkerType::Chapter {
                self.renumber_chapters();
            }
            
            Some(marker)
        } else {
            None
        }
    }

    pub fn get(&self, marker_id: MarkerId) -> Option<&Marker> {
        self.by_id.get(&marker_id)
    }

    pub fn get_mut(&mut self, marker_id: MarkerId) -> Option<&mut Marker> {
        self.by_id.get_mut(&marker_id)
    }

    pub fn at_frame(&self, frame: Frame) -> Vec<&Marker> {
        self.markers.get(&frame).map(|v| v.iter().collect()).unwrap_or_default()
    }

    pub fn in_range(&self, range: FrameRange) -> Vec<&Marker> {
        self.markers.range(range.start..range.end)
            .flat_map(|(_, v)| v.iter())
            .collect()
    }

    pub fn all(&self) -> Vec<&Marker> {
        self.by_id.values().collect()
    }

    pub fn next_marker(&self, frame: Frame) -> Option<&Marker> {
        self.markers.range((frame + Frame(1))..)
            .next()
            .and_then(|(_, v)| v.first())
    }

    pub fn prev_marker(&self, frame: Frame) -> Option<&Marker> {
        self.markers.range(..frame)
            .next_back()
            .and_then(|(_, v)| v.last())
    }

    pub fn next_chapter(&self, frame: Frame) -> Option<&Marker> {
        self.markers.range((frame + Frame(1))..)
            .find_map(|(_, v)| v.iter().find(|m| m.marker_type == MarkerType::Chapter))
    }

    pub fn prev_chapter(&self, frame: Frame) -> Option<&Marker> {
        self.markers.range(..frame)
            .next_back()
            .and_then(|(_, v)| v.iter().rev().find(|m| m.marker_type == MarkerType::Chapter))
    }

    pub fn update(&mut self, marker_id: MarkerId, f: impl FnOnce(&mut Marker)) -> bool {
        if let Some(marker) = self.by_id.get_mut(&marker_id) {
            let old_position = marker.position;
            f(marker);
            
            if marker.position != old_position {
                // Move to new position
                if let Some(vec) = self.markers.get_mut(&old_position) {
                    vec.retain(|m| m.id != marker_id);
                    if vec.is_empty() {
                        self.markers.remove(&old_position);
                    }
                }
                self.markers.entry(marker.position).or_default().push(marker.clone());
            }
            
            if marker.marker_type == MarkerType::Chapter {
                self.renumber_chapters();
            }
            
            true
        } else {
            false
        }
    }

    pub fn set_chapter(&mut self, marker_id: MarkerId, is_chapter: bool) -> bool {
        if let Some(marker) = self.by_id.get_mut(&marker_id) {
            if is_chapter && marker.marker_type != MarkerType::Chapter {
                marker.marker_type = MarkerType::Chapter;
                marker.chapter_number = Some(self.next_chapter);
                self.next_chapter += 1;
            } else if !is_chapter && marker.marker_type == MarkerType::Chapter {
                marker.marker_type = MarkerType::Note;
                marker.chapter_number = None;
                self.renumber_chapters();
            }
            true
        } else { false }
    }

    fn renumber_chapters(&mut self) {
        self.next_chapter = 1;
        for marker in self.by_id.values_mut() {
            if marker.marker_type == MarkerType::Chapter {
                marker.chapter_number = Some(self.next_chapter);
                self.next_chapter += 1;
            }
        }
    }

    pub fn clear(&mut self) {
        self.markers.clear();
        self.by_id.clear();
        self.next_chapter = 1;
    }

    pub fn count(&self) -> usize { self.by_id.len() }

    pub fn export_chapters(&self) -> Vec<ChapterExport> {
        self.by_id.values()
            .filter(|m| m.marker_type == MarkerType::Chapter)
            .map(|m| ChapterExport {
                number: m.chapter_number.unwrap_or(0),
                name: m.name.clone(),
                start_timecode: Timecode::from_frame(m.position, FrameRate::Fps24),
                start_frame: m.position,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ChapterExport {
    pub number: u32,
    pub name: String,
    pub start_timecode: Timecode,
    pub start_frame: Frame,
}

/// Marker UI helpers
pub struct MarkerUI {
    pub show_markers: bool,
    pub show_chapters: bool,
    pub show_notes: bool,
    pub show_todos: bool,
    pub show_sync: bool,
    pub filter_color: Option<ColorLabel>,
}

impl Default for MarkerUI {
    fn default() -> Self {
        Self {
            show_markers: true,
            show_chapters: true,
            show_notes: true,
            show_todos: true,
            show_sync: true,
            filter_color: None,
        }
    }
}

impl MarkerUI {
    pub fn filter(&self, marker: &Marker) -> bool {
        if !self.show_markers { return false; }
        let type_visible = match marker.marker_type {
            MarkerType::Chapter => self.show_chapters,
            MarkerType::Note => self.show_notes,
            MarkerType::Todo => self.show_todos,
            MarkerType::SyncPoint => self.show_sync,
            _ => true,
        };
        type_visible && self.filter_color.map_or(true, |c| c == marker.color)
    }
}