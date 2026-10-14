//! Core types and architecture for DaVinci Clone
//! Minimal, zero-cost abstractions for video editing

pub mod types;
pub mod time;
pub mod color;
pub mod math;
pub mod id;
pub mod error;
pub mod config;

pub use types::*;
pub use time::*;
pub use color::*;
pub use math::*;
pub use id::*;
pub use error::*;
pub use config::*;