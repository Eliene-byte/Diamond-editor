//! DaVinci Clone - Lightweight DaVinci Resolve clone in Rust
//! Edit page focused, ~45MB target binary

mod core;
mod media;
mod timeline;
mod effects;
mod gpu;
mod audio;
mod ui;

use core::config::{AppConfig, ConfigManager};
use core::error::Result;
use media::pool::MediaPool;
use timeline::clip::Timeline;
use gpu::renderer::{GpuRenderer, RendererBackend};
use ui::pages::{CutPage, EditPage, ColorPage, FusionPage, FairlightPage, DeliverPage};
use ui::theme::Theme;
use ui::layout::LayoutContext;
use std::sync::{Arc, RwLock};
use std::time::Instant;

const APP_NAME: &str = "DaVinci Clone";
const VERSION: &str = "0.1.0";

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load configuration
    let config_manager = ConfigManager::new()?;
    let config = config_manager.get().clone();
    
    println!("{} v{}", APP_NAME, VERSION);
    println!("Configuration loaded");
    
    // Initialize GPU renderer with automatic fallback
    let runtime = tokio::runtime::Runtime::new()?;
    let renderer = runtime.block_on(async {
        GpuRenderer::with_backend(1920, 1080, config.gpu.vsync.into(), config.gpu.vsync, None).await
    })?;
    println!("Renderer backend: {:?}", renderer.backend());
    let renderer = Arc::new(RwLock::new(renderer));
    
    // Create media pool
    let media_pool = Arc::new(RwLock::new(MediaPool::new(
        core::id::ProjectId::generate(),
        Arc::new(crate::media::cache::MediaCache::new(crate::media::cache::CacheConfig::default()))?,
    )));
    
    // Create timeline
    let timeline = Arc::new(RwLock::new(Timeline::new(
        core::id::ProjectId::generate(),
        "Timeline 1".into(),
        config.timeline.clone().into(),
    )));
    
    // Initialize pages
    let mut cut_page = CutPage::new();
    cut_page.set_timeline(timeline.clone());
    cut_page.set_media_pool(media_pool.clone());
    
    let mut edit_page = EditPage::new();
    edit_page.set_timeline(timeline.clone());
    edit_page.set_media_pool(media_pool.clone());
    
    let mut color_page = ColorPage::new();
    color_page.set_timeline(timeline.clone());
    
    let mut fusion_page = FusionPage::new();
    fusion_page.set_timeline(timeline.clone());
    
    let mut fairlight_page = FairlightPage::new();
    fairlight_page.set_timeline(timeline.clone());
    
    let mut deliver_page = DeliverPage::new();
    deliver_page.set_timeline(timeline.clone());
    
    // Current page
    let mut current_page = Page::Edit;
    
    // Initialize window
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = winit::window::WindowBuilder::new()
        .with_title(APP_NAME)
        .with_inner_size(winit::dpi::LogicalSize::new(1920, 1080))
        .build(&event_loop)?;
    
    // Configure renderer surface
    let surface = renderer.read().device().create_surface(&window)?;
    renderer.write().configure_surface(surface, 1920, 1080);
    
    // Theme
    let theme = Theme::default();
    let mut layout_ctx = LayoutContext::new(theme);
    
    // Main loop
    let mut last_frame = Instant::now();
    let mut frame_count = 0;
    
    event_loop.run(move |event, control_flow| {
        control_flow.set_poll();
        
        match event {
            winit::event::Event::WindowEvent { event, .. } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        control_flow.exit();
                    }
                    winit::event::WindowEvent::Resized(size) => {
                        renderer.write().resize(size.width, size.height);
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let delta = now.duration_since(last_frame).as_secs_f32();
                        last_frame = now;
                        frame_count += 1;
                        
                        // Update layout context
                        layout_ctx.scale = window.scale_factor() as f32;
                        
                        // Render current page
                        let page_widget = match current_page {
                            Page::Cut => cut_page.ui(&layout_ctx),
                            Page::Edit => edit_page.ui(&layout_ctx),
                            Page::Color => color_page.ui(&layout_ctx),
                            Page::Fusion => fusion_page.ui(&layout_ctx),
                            Page::Fairlight => fairlight_page.ui(&layout_ctx),
                            Page::Deliver => deliver_page.ui(&layout_ctx),
                        };
                        
                        // Render frame
                        if let Ok(mut renderer) = renderer.write() {
                            let _ = renderer.render_frame(timeline.read().unwrap().playhead);
                        }
                        
                        window.request_redraw();
                    }
                    winit::event::WindowEvent::KeyboardInput { event, .. } => {
                        // Handle keyboard shortcuts
                        if let winit::event::ElementState::Pressed = event.state {
                            match event.physical_key {
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Digit1) => current_page = Page::Cut,
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Digit2) => current_page = Page::Edit,
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Digit3) => current_page = Page::Color,
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Digit4) => current_page = Page::Fusion,
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Digit5) => current_page = Page::Fairlight,
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Digit6) => current_page = Page::Deliver,
                                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Space) => {
                                    // Toggle playback
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            winit::event::Event::AboutToWait => {
                // Update logic here
            }
            _ => {}
        }
    })?;
    
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Cut,
    Edit,
    Color,
    Fusion,
    Fairlight,
    Deliver,
}

impl From<bool> for wgpu::PresentMode {
    fn from(vsync: bool) -> Self {
        if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::Immediate }
    }
}