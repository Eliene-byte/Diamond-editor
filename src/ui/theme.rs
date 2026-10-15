//! UI Theme - DaVinci Resolve dark theme

use crate::core::math::Rect;

/// Theme colors matching DaVinci Resolve
pub struct Theme {
    pub bg_primary: [f32; 4],
    pub bg_secondary: [f32; 4],
    pub bg_tertiary: [f32; 4],
    pub bg_hover: [f32; 4],
    pub bg_active: [f32; 4],
    pub bg_selected: [f32; 4],
    pub bg_focus: [f32; 4],
    pub text_primary: [f32; 4],
    pub text_secondary: [f32; 4],
    pub text_disabled: [f32; 4],
    pub text_accent: [f32; 4],
    pub border_subtle: [f32; 4],
    pub border_normal: [f32; 4],
    pub border_focus: [f32; 4],
    pub border_error: [f32; 4],
    pub accent: [f32; 4],
    pub accent_hover: [f32; 4],
    pub accent_active: [f32; 4],
    pub success: [f32; 4],
    pub warning: [f32; 4],
    pub error: [f32; 4],
    pub info: [f32; 4],
    pub video_track: [f32; 4],
    pub audio_track: [f32; 4],
    pub subtitle_track: [f32; 4],
    pub clip_video: [f32; 4],
    pub clip_audio: [f32; 4],
    pub clip_image: [f32; 4],
    pub clip_compound: [f32; 4],
    pub clip_generator: [f32; 4],
    pub label_red: [f32; 4],
    pub label_orange: [f32; 4],
    pub label_yellow: [f32; 4],
    pub label_green: [f32; 4],
    pub label_cyan: [f32; 4],
    pub label_blue: [f32; 4],
    pub label_purple: [f32; 4],
    pub label_pink: [f32; 4],
    pub spacing_xs: f32,
    pub spacing_sm: f32,
    pub spacing_md: f32,
    pub spacing_lg: f32,
    pub spacing_xl: f32,
    pub button_height: f32,
    pub button_height_sm: f32,
    pub button_height_lg: f32,
    pub input_height: f32,
    pub slider_height: f32,
    pub header_height: f32,
    pub timeline_track_height: f32,
    pub timeline_audio_height: f32,
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub radius_full: f32,
    pub shadow_sm: Shadow,
    pub shadow_md: Shadow,
    pub shadow_lg: Shadow,
    pub font_family: String,
    pub font_size_xs: f32,
    pub font_size_sm: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    pub font_size_title: f32,
    pub font_weight_normal: u32,
    pub font_weight_medium: u32,
    pub font_weight_bold: u32,
    pub animation_fast: f32,
    pub animation_normal: f32,
    pub animation_slow: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Shadow {
    pub offset: [f32; 2],
    pub blur: f32,
    pub color: [f32; 4],
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg_primary: hex("#1e1e1e"),
            bg_secondary: hex("#252525"),
            bg_tertiary: hex("#2d2d2d"),
            bg_hover: hex("#333333"),
            bg_active: hex("#3d3d3d"),
            bg_selected: hex("#006bb3"),
            bg_focus: hex("#0078d4"),
            text_primary: hex("#ffffff"),
            text_secondary: hex("#cccccc"),
            text_disabled: hex("#888888"),
            text_accent: hex("#ff6b00"),
            border_subtle: hex("#333333"),
            border_normal: hex("#444444"),
            border_focus: hex("#0078d4"),
            border_error: hex("#e81123"),
            accent: hex("#ff6b00"),
            accent_hover: hex("#ff8533"),
            accent_active: hex("#cc5500"),
            success: hex("#107c10"),
            warning: hex("#ffb900"),
            error: hex("#e81123"),
            info: hex("#0078d4"),
            video_track: hex("#3a3a4a"),
            audio_track: hex("#3a4a3a"),
            subtitle_track: hex("#4a3a3a"),
            clip_video: hex("#4a5a7a"),
            clip_audio: hex("#5a7a4a"),
            clip_image: hex("#7a6a4a"),
            clip_compound: hex("#6a4a7a"),
            clip_generator: hex("#4a7a7a"),
            label_red: hex("#e81123"),
            label_orange: hex("#ea7700"),
            label_yellow: hex("#ffb900"),
            label_green: hex("#107c10"),
            label_cyan: hex("#00b7c3"),
            label_blue: hex("#0078d4"),
            label_purple: hex("#881798"),
            label_pink: hex("#e3008c"),
            spacing_xs: 2.0,
            spacing_sm: 4.0,
            spacing_md: 8.0,
            spacing_lg: 16.0,
            spacing_xl: 24.0,
            button_height: 28.0,
            button_height_sm: 22.0,
            button_height_lg: 36.0,
            input_height: 28.0,
            slider_height: 20.0,
            header_height: 32.0,
            timeline_track_height: 60.0,
            timeline_audio_height: 40.0,
            radius_sm: 2.0,
            radius_md: 4.0,
            radius_lg: 8.0,
            radius_full: 9999.0,
            shadow_sm: Shadow {
                offset: [0.0, 1.0],
                blur: 2.0,
                color: rgba(0.0, 0.0, 0.0, 0.3),
            },
            shadow_md: Shadow {
                offset: [0.0, 4.0],
                blur: 8.0,
                color: rgba(0.0, 0.0, 0.0, 0.4),
            },
            shadow_lg: Shadow {
                offset: [0.0, 8.0],
                blur: 16.0,
                color: rgba(0.0, 0.0, 0.0, 0.5),
            },
            font_family: "Inter".into(),
            font_size_xs: 11.0,
            font_size_sm: 12.0,
            font_size_md: 13.0,
            font_size_lg: 14.0,
            font_size_xl: 16.0,
            font_size_title: 20.0,
            font_weight_normal: 400,
            font_weight_medium: 500,
            font_weight_bold: 600,
            animation_fast: 100.0,
            animation_normal: 200.0,
            animation_slow: 300.0,
        }
    }
}

impl Theme {
    pub fn light() -> Self {
        let mut t = Self::default();
        t.bg_primary = hex("#ffffff");
        t.bg_secondary = hex("#f3f2f1");
        t.bg_tertiary = hex("#e1dfdd");
        t.bg_hover = hex("#e1dfdd");
        t.bg_active = hex("#d2d0ce");
        t.bg_selected = hex("#0078d4");
        t.bg_focus = hex("#0078d4");
        t.text_primary = hex("#323130");
        t.text_secondary = hex("#605e5c");
        t.text_disabled = hex("#a19f9d");
        t.border_subtle = hex("#d2d0ce");
        t.border_normal = hex("#c8c6c4");
        t.border_focus = hex("#0078d4");
        t.video_track = hex("#e8ecf4");
        t.audio_track = hex("#e8f4e8");
        t.subtitle_track = hex("#f4e8e8");
        t.clip_video = hex("#d6e4f0");
        t.clip_audio = hex("#e4f0d6");
        t.clip_image = hex("#f0ead6");
        t.clip_compound = hex("#ead6f0");
        t.clip_generator = hex("#d6f0f0");
        t
    }

    pub fn high_contrast() -> Self {
        let mut t = Self::default();
        t.bg_primary = hex("#000000");
        t.bg_secondary = hex("#1a1a1a");
        t.bg_tertiary = hex("#333333");
        t.text_primary = hex("#ffffff");
        t.text_secondary = hex("#ffff00");
        t.border_normal = hex("#ffffff");
        t.accent = hex("#ffff00");
        t
    }

    pub fn clip_color_for_type(&self, media_type: crate::core::types::MediaType) -> [f32; 4] {
        match media_type {
            crate::core::types::MediaType::Video => self.clip_video,
            crate::core::types::MediaType::Audio => self.clip_audio,
            crate::core::types::MediaType::Image => self.clip_image,
            crate::core::types::MediaType::Compound => self.clip_compound,
            crate::core::types::MediaType::Generator => self.clip_generator,
            _ => self.clip_video,
        }
    }

    pub fn label_color(&self, label: crate::core::types::ColorLabel) -> [f32; 4] {
        match label {
            crate::core::types::ColorLabel::Red => self.label_red,
            crate::core::types::ColorLabel::Orange => self.label_orange,
            crate::core::types::ColorLabel::Yellow => self.label_yellow,
            crate::core::types::ColorLabel::Green => self.label_green,
            crate::core::types::ColorLabel::Cyan => self.label_cyan,
            crate::core::types::ColorLabel::Blue => self.label_blue,
            crate::core::types::ColorLabel::Purple => self.label_purple,
            crate::core::types::ColorLabel::Pink => self.label_pink,
            _ => self.text_disabled,
        }
    }
}

fn hex(s: &str) -> [f32; 4] {
    let s = s.trim_start_matches('#');
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0) as f32 / 255.0;
    [r, g, b, 1.0]
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r, g, b, a]
}
