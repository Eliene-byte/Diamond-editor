//! Custom lightweight UI framework for DaVinci Clone
//! No egui dependency - custom immediate mode on wgpu for minimum size

pub mod widgets;
pub mod layout;
pub mod theme;
pub mod input;
pub mod pages;
pub mod timeline_widget;
pub mod viewer;
pub mod scopes_widget;
pub mod mixer;
pub mod inspector;
pub mod media_pool_widget;
pub mod node_graph;

pub use widgets::*;
pub use layout::*;
pub use theme::*;
pub use input::*;
pub use pages::*;