//! Typed IDs for all entities
//! Zero-cost abstractions with compile-time type safety

use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// Generate unique IDs
static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u64);

        impl $name {
            pub const fn new(id: u64) -> Self { Self(id) }
            pub const fn nil() -> Self { Self(0) }
            pub fn generate() -> Self { Self(next_id()) }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}_{:016x}", stringify!($name).to_lowercase(), self.0)
            }
        }

        impl From<u64> for $name {
            fn from(id: u64) -> Self { Self(id) }
        }

        impl From<$name> for u64 {
            fn from(id: $name) -> u64 { id.0 }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: serde::Serializer {
                serializer.serialize_u64(self.0)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
                Ok(Self(u64::deserialize(deserializer)?))
            }
        }
    };
}

// Project & Timeline
typed_id!(ProjectId);
typed_id!(TimelineId);
typed_id!(BinId);

// Media
typed_id!(ClipId);
typed_id!(MediaId);
typed_id!(ProxyId);
typed_id!(ThumbnailId);

// Tracks & Clips on Timeline
typed_id!(TrackId);
typed_id!(TrackItemId);
typed_id!(TransitionId);

// Effects & Grading
typed_id!(EffectId);
typed_id!(GradeNodeId);
typed_id!(LutId);
typed_id!(PluginId);

// Markers & Annotations
typed_id!(MarkerId);
typed_id!(AnnotationId);

// Audio
typed_id!(AudioBusId);
typed_id!(AudioPluginId);

// Export/Render
typed_id!(RenderJobId);
typed_id!(ExportPresetId);

// UI
typed_id!(WindowId);
typed_id!(PanelId);
typed_id!(WidgetId);

// Cache
typed_id!(CacheEntryId);

// Commands/Undo
typed_id!(CommandId);

/// Compound ID for objects that can be in multiple contexts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompoundId {
    pub project: ProjectId,
    pub timeline: TimelineId,
    pub object: ObjectId,
}

/// Generic object ID (for selection, commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u64);

impl ObjectId {
    pub const fn new(id: u64) -> Self { Self(id) }
    pub const fn nil() -> Self { Self(0) }
    pub fn generate() -> Self { Self(next_id()) }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "obj_{:016x}", self.0)
    }
}

impl From<ClipId> for ObjectId { fn from(id: ClipId) -> Self { Self(id.0) } }
impl From<TrackId> for ObjectId { fn from(id: TrackId) -> Self { Self(id.0) } }
impl From<TrackItemId> for ObjectId { fn from(id: TrackItemId) -> Self { Self(id.0) } }
impl From<EffectId> for ObjectId { fn from(id: EffectId) -> Self { Self(id.0) } }
impl From<MarkerId> for ObjectId { fn from(id: MarkerId) -> Self { Self(id.0) } }
impl From<GradeNodeId> for ObjectId { fn from(id: GradeNodeId) -> Self { Self(id.0) } }
impl From<TransitionId> for ObjectId { fn from(id: TransitionId) -> Self { Self(id.0) } }

/// ID allocator for a specific type
pub struct IdAllocator<T> {
    next: u64,
    free: Vec<u64>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Default for IdAllocator<T> {
    fn default() -> Self { Self { next: 1, free: Vec::new(), _phantom: std::marker::PhantomData } }
}

impl<T> IdAllocator<T> {
    pub fn allocate(&mut self) -> u64 {
        self.free.pop().unwrap_or_else(|| { let id = self.next; self.next += 1; id })
    }
    pub fn deallocate(&mut self, id: u64) { self.free.push(id); }
    pub fn allocated_count(&self) -> usize { (self.next - 1) as usize - self.free.len() }
}

/// ID map - efficient storage for ID -> value mappings
pub mod id_map {
    use super::*;
    use std::collections::HashMap;

    pub struct IdMap<K, V> {
        map: HashMap<K, V>,
    }

    impl<K: Eq + Hash + Copy, V> Default for IdMap<K, V> {
        fn default() -> Self { Self { map: HashMap::default() } }
    }

    impl<K: Eq + Hash + Copy, V> IdMap<K, V> {
        pub fn new() -> Self { Self::default() }
        pub fn insert(&mut self, key: K, value: V) -> Option<V> { self.map.insert(key, value) }
        pub fn get(&self, key: K) -> Option<&V> { self.map.get(&key) }
        pub fn get_mut(&mut self, key: K) -> Option<&mut V> { self.map.get_mut(&key) }
        pub fn remove(&mut self, key: K) -> Option<V> { self.map.remove(&key) }
        pub fn contains(&self, key: K) -> bool { self.map.contains_key(&key) }
        pub fn len(&self) -> usize { self.map.len() }
        pub fn is_empty(&self) -> bool { self.map.is_empty() }
        pub fn clear(&mut self) { self.map.clear(); }
        pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> { self.map.iter() }
        pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> { self.map.iter_mut() }
        pub fn keys(&self) -> impl Iterator<Item = &K> { self.map.keys() }
        pub fn values(&self) -> impl Iterator<Item = &V> { self.map.values() }
        pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> { self.map.values_mut() }
    }
}