//! DaVinci Clone - Lightweight DaVinci Resolve clone in Rust

pub mod audio;
pub mod core;
pub mod effects;
pub mod gpu;
pub mod media;
pub mod timeline;
pub mod ui;

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        assert_eq!(2 + 2, 4);
    }
}
