//! DaVinci Clone - Lightweight DaVinci Resolve clone in Rust
//! Edit page focused, ~45MB target binary.

use davinci_clone::core::config::ConfigManager;
use davinci_clone::core::error::Result;
use davinci_clone::core::id::ProjectId;
use davinci_clone::gpu::renderer::GpuRenderer;
use davinci_clone::media::cache::{CacheConfig, MediaCache};
use davinci_clone::media::pool::MediaPool;
use davinci_clone::timeline::clip::{Timeline, TimelineSettings};
use davinci_clone::ui::layout::LayoutContext;
use davinci_clone::ui::pages::{
    ColorPage, CutPage, DeliverPage, EditPage, FairlightPage, FusionPage,
};
use davinci_clone::ui::theme::Theme;
use std::sync::{Arc, RwLock};
use std::time::Instant;

const APP_NAME: &str = "DaVinci Clone";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The six DaVinci-style pages, in tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Cut,
    Edit,
    Color,
    Fusion,
    Fairlight,
    Deliver,
}

impl Page {
    const ALL: [Page; 6] = [
        Page::Cut,
        Page::Edit,
        Page::Color,
        Page::Fusion,
        Page::Fairlight,
        Page::Deliver,
    ];
}

fn main() -> Result<()> {
    println!("{APP_NAME} v{VERSION}");

    // Load configuration, falling back to defaults when none is on disk.
    let config_manager = match ConfigManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("could not read config ({e}); using defaults");
            ConfigManager::in_memory_default()
        }
    };
    let config = config_manager.get().clone();
    println!("configuration loaded");

    // Media pool and its cache.
    let cache = MediaCache::new(CacheConfig::default())?;
    let media_pool = Arc::new(RwLock::new(MediaPool::new(
        ProjectId::generate(),
        Arc::new(cache),
    )));

    // A single timeline drives every page, matching Resolve's project model.
    let timeline_settings = TimelineSettings {
        video_track_count: config.timeline.default_video_tracks,
        audio_track_count: config.timeline.default_audio_tracks,
        ..Default::default()
    };
    let timeline = Arc::new(RwLock::new(Timeline::new(
        ProjectId::generate(),
        "Timeline 1".into(),
        timeline_settings,
    )));

    // One tokio runtime for async work that is not the winit event loop.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // Window + renderer. `with_backend` walks Vulkan -> Metal -> DX12 -> CPU,
    // so this succeeds even on a machine with no usable GPU driver.
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let window = Arc::new(
        event_loop.create_window(
            winit::window::WindowAttributes::default()
                .with_title(format!("{APP_NAME} v{VERSION}"))
                .with_inner_size(winit::dpi::LogicalSize::new(1920, 1080)),
        )?,
    );

    let present_mode = if config.gpu.vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::Immediate
    };

    let renderer = runtime.block_on(GpuRenderer::with_backend(
        1920,
        1080,
        present_mode,
        config.gpu.vsync,
        None,
    ))?;
    println!("renderer backend: {:?}", renderer.backend());
    let renderer = Arc::new(RwLock::new(renderer));

    // Pages
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

    let mut current_page = Page::Edit;
    let mut layout_ctx = LayoutContext::new(Theme::default());
    let mut last_frame = Instant::now();
    let mut frame_count: u64 = 0;

    event_loop.run(move |event, control_flow| {
        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => control_flow.exit(),

                winit::event::WindowEvent::Resized(size) => {
                    if let Ok(mut r) = renderer.write() {
                        r.resize(size.width, size.height);
                    }
                }

                winit::event::WindowEvent::RedrawRequested => {
                    let now = Instant::now();
                    let delta = now.duration_since(last_frame).as_secs_f32();
                    last_frame = now;
                    frame_count += 1;

                    if frame_count % 120 == 0 {
                        println!("frame {frame_count}, last delta {delta:.3}s");
                    }

                    layout_ctx.scale = window.scale_factor() as f32;

                    // Build the active page's widget tree. The pages own their
                    // widgets and rebuild the tree whenever the layout changes,
                    // so this is cheap relative to a frame.
                    let page_widget = match current_page {
                        Page::Cut => cut_page.ui(&layout_ctx),
                        Page::Edit => edit_page.ui(&layout_ctx),
                        Page::Color => color_page.ui(&layout_ctx),
                        Page::Fusion => fusion_page.ui(&layout_ctx),
                        Page::Fairlight => fairlight_page.ui(&layout_ctx),
                        Page::Deliver => deliver_page.ui(&layout_ctx),
                    };

                    if let Ok(mut r) = renderer.write() {
                        let playhead = timeline.read().map(|tl| tl.playhead).unwrap_or_default();
                        let _ = page_widget;
                        let _ = r.render_frame(playhead);
                    }

                    window.request_redraw();
                }

                winit::event::WindowEvent::KeyboardInput { event, .. } => {
                    use winit::event::ElementState;
                    use winit::keyboard::KeyCode;
                    use winit::keyboard::PhysicalKey;

                    if event.state != ElementState::Pressed {
                        return;
                    }

                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Digit1) => current_page = Page::Cut,
                        PhysicalKey::Code(KeyCode::Digit2) => current_page = Page::Edit,
                        PhysicalKey::Code(KeyCode::Digit3) => current_page = Page::Color,
                        PhysicalKey::Code(KeyCode::Digit4) => current_page = Page::Fusion,
                        PhysicalKey::Code(KeyCode::Digit5) => current_page = Page::Fairlight,
                        PhysicalKey::Code(KeyCode::Digit6) => current_page = Page::Deliver,
                        PhysicalKey::Code(KeyCode::Escape) => control_flow.exit(),
                        _ => {}
                    }
                }

                _ => {}
            },

            _ => {}
        }
    })?;

    // `Page::ALL` is only referenced here so the page list stays in one place.
    debug_assert_eq!(Page::ALL.len(), 6);

    Ok(())
}
