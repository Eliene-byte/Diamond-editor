//! DaVinci Clone - Lightweight DaVinci Resolve clone in Rust

pub mod core;
pub mod media;
pub mod timeline;
pub mod effects;
pub mod gpu;
pub mod audio;
pub mod ui;

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        assert_eq!(2 + 2, 4);
    }
}