//! Color science and pixel formats
//! Professional color management for grading

use std::fmt;

/// Color space / transfer function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    // Scene-referred
    Linear,
    LogC,
    SLog2,
    SLog3,
    CLog,
    CLog2,
    CLog3,
    VLog,
    ArriLogC,
    RedLog3G10,
    BmdFilm,
    BmdFilmGen5,
    // Display-referred
    Rec709,
    Rec2020,
    Rec2100_PQ,
    Rec2100_HLG,
    P3D65,
    P3DCI,
    sRGB,
    AdobeRGB,
    // ACES
    ACES2065_1,
    ACEScct,
    ACEScg,
    // Custom
    Custom,
}

impl ColorSpace {
    pub const fn is_linear(self) -> bool {
        matches!(self, Self::Linear | Self::ACES2065_1 | Self::ACEScg)
    }
    pub const fn is_log(self) -> bool {
        matches!(self, Self::LogC | Self::SLog2 | Self::SLog3 | Self::CLog | Self::CLog2
            | Self::CLog3 | Self::VLog | Self::ArriLogC | Self::RedLog3G10
            | Self::BmdFilm | Self::BmdFilmGen5 | Self::ACEScct)
    }
    pub const fn is_hdr(self) -> bool {
        matches!(self, Self::Rec2100_PQ | Self::Rec2100_HLG | Self::P3D65 | Self::P3DCI)
    }
    pub const fn primaries(self) -> Primaries {
        match self {
            Self::Rec709 | Self::sRGB => Primaries::BT709,
            Self::Rec2020 | Self::Rec2100_PQ | Self::Rec2100_HLG => Primaries::BT2020,
            Self::P3D65 | Self::P3DCI => Primaries::P3,
            Self::ACES2065_1 | Self::ACEScct | Self::ACEScg => Primaries::ACES,
            Self::AdobeRGB => Primaries::AdobeRGB,
            _ => Primaries::BT709,
        }
    }
    pub const fn transfer(self) -> TransferFunction {
        match self {
            Self::Linear | Self::ACES2065_1 | Self::ACEScg => TransferFunction::Linear,
            Self::Rec709 => TransferFunction::BT709,
            Self::sRGB => TransferFunction::SRGB,
            Self::Rec2020 => TransferFunction::BT2020,
            Self::Rec2100_PQ => TransferFunction::PQ,
            Self::Rec2100_HLG => TransferFunction::HLG,
            Self::P3D65 | Self::P3DCI => TransferFunction::P3,
            Self::AdobeRGB => TransferFunction::AdobeRGB,
            Self::ACEScct => TransferFunction::ACEScct,
            Self::LogC => TransferFunction::LogC,
            Self::SLog2 => TransferFunction::SLog2,
            Self::SLog3 => TransferFunction::SLog3,
            Self::CLog => TransferFunction::CLog,
            Self::CLog2 => TransferFunction::CLog2,
            Self::CLog3 => TransferFunction::CLog3,
            Self::VLog => TransferFunction::VLog,
            Self::ArriLogC => TransferFunction::ArriLogC,
            Self::RedLog3G10 => TransferFunction::RedLog3G10,
            Self::BmdFilm => TransferFunction::BmdFilm,
            Self::BmdFilmGen5 => TransferFunction::BmdFilmGen5,
            Self::Custom => TransferFunction::Linear,
        }
    }
}

/// Chromaticity primaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primaries {
    BT709,
    BT2020,
    P3,
    ACES,
    AdobeRGB,
    Custom,
}

/// Transfer function (OETF/EOTF)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransferFunction {
    Linear,
    BT709,
    BT2020,
    SRGB,
    PQ,
    HLG,
    P3,
    AdobeRGB,
    ACEScct,
    LogC,
    SLog2,
    SLog3,
    CLog,
    CLog2,
    CLog3,
    VLog,
    ArriLogC,
    RedLog3G10,
    BmdFilm,
    BmdFilmGen5,
    Custom,
}

/// Pixel format / bit depth
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    // 8-bit
    Rgb8,
    Rgba8,
    Yuv420_8,
    Yuv422_8,
    Yuv444_8,
    // 10-bit
    Rgb10,
    Rgba10,
    Yuv420_10,
    Yuv422_10,
    Yuv444_10,
    P010,
    // 12-bit
    Rgb12,
    Rgba12,
    Yuv420_12,
    Yuv422_12,
    Yuv444_12,
    // 16-bit float
    Rgba16F,
    Rgb16F,
    // 32-bit float
    Rgba32F,
    Rgb32F,
    // Packed
    Yuy2,
    UyvY,
    V210,
    V410,
}

impl PixelFormat {
    pub const fn bits_per_pixel(self) -> u8 {
        match self {
            Self::Rgb8 | Self::Yuv420_8 | Self::Yuv422_8 => 24,
            Self::Rgba8 | Self::Yuv444_8 => 32,
            Self::Rgb10 | Self::Yuv420_10 | Self::Yuv422_10 => 30,
            Self::Rgba10 | Self::Yuv444_10 | Self::P010 => 40,
            Self::Rgb12 | Self::Yuv420_12 | Self::Yuv422_12 => 36,
            Self::Rgba12 | Self::Yuv444_12 => 48,
            Self::Rgb16F => 48,
            Self::Rgba16F => 64,
            Self::Rgb32F => 96,
            Self::Rgba32F => 128,
            Self::Yuy2 | Self::UyvY => 16,
            Self::V210 => 32,
            Self::V410 => 64,
        }
    }
    pub const fn channels(self) -> u8 {
        match self {
            Self::Rgb8 | Self::Rgb10 | Self::Rgb12 | Self::Rgb16F | Self::Rgb32F => 3,
            Self::Rgba8 | Self::Rgba10 | Self::Rgba12 | Self::Rgba16F | Self::Rgba32F => 4,
            Self::Yuv420_8 | Self::Yuv422_8 | Self::Yuv444_8
            | Self::Yuv420_10 | Self::Yuv422_10 | Self::Yuv444_10
            | Self::Yuv420_12 | Self::Yuv422_12 | Self::Yuv444_12 => 3,
            Self::P010 => 3,
            Self::Yuy2 | Self::UyvY | Self::V210 | Self::V410 => 3,
        }
    }
    pub const fn is_float(self) -> bool {
        matches!(self, Self::Rgba16F | Self::Rgb16F | Self::Rgba32F | Self::Rgb32F)
    }
    pub const fn is_yuv(self) -> bool {
        matches!(self, Self::Yuv420_8 | Self::Yuv422_8 | Self::Yuv444_8
            | Self::Yuv420_10 | Self::Yuv422_10 | Self::Yuv444_10
            | Self::Yuv420_12 | Self::Yuv422_12 | Self::Yuv444_12
            | Self::P010 | Self::Yuy2 | Self::UyvY | Self::V210 | Self::V410)
    }
    pub const fn is_planar(self) -> bool {
        matches!(self, Self::Yuv420_8 | Self::Yuv422_8 | Self::Yuv444_8
            | Self::Yuv420_10 | Self::Yuv422_10 | Self::Yuv444_10
            | Self::Yuv420_12 | Self::Yuv422_12 | Self::Yuv444_12
            | Self::P010)
    }
}

/// Color transform (3x3 matrix + offset)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorTransform {
    pub matrix: [[f32; 3]; 3],
    pub offset: [f32; 3],
}

impl ColorTransform {
    pub const IDENTITY: Self = Self {
        matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        offset: [0.0, 0.0, 0.0],
    };

    pub fn multiply(self, other: Self) -> Self {
        let mut m = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] = self.matrix[i][0] * other.matrix[0][j]
                    + self.matrix[i][1] * other.matrix[1][j]
                    + self.matrix[i][2] * other.matrix[2][j];
            }
        }
        let o = [
            self.matrix[0][0] * other.offset[0] + self.matrix[0][1] * other.offset[1] + self.matrix[0][2] * other.offset[2] + self.offset[0],
            self.matrix[1][0] * other.offset[0] + self.matrix[1][1] * other.offset[1] + self.matrix[1][2] * other.offset[2] + self.offset[1],
            self.matrix[2][0] * other.offset[0] + self.matrix[2][1] * other.offset[1] + self.matrix[2][2] * other.offset[2] + self.offset[2],
        ];
        Self { matrix: m, offset: o }
    }

    pub fn apply(self, rgb: [f32; 3]) -> [f32; 3] {
        [
            self.matrix[0][0] * rgb[0] + self.matrix[0][1] * rgb[1] + self.matrix[0][2] * rgb[2] + self.offset[0],
            self.matrix[1][0] * rgb[0] + self.matrix[1][1] * rgb[1] + self.matrix[1][2] * rgb[2] + self.offset[1],
            self.matrix[2][0] * rgb[0] + self.matrix[2][1] * rgb[1] + self.matrix[2][2] * rgb[2] + self.offset[2],
        ]
    }
}

/// 1D LUT (shaper)
#[derive(Debug, Clone)]
pub struct Lut1D {
    pub size: usize,
    pub data: Vec<[f32; 3]>,
    pub input_range: (f32, f32),
    pub output_range: (f32, f32),
}

impl Lut1D {
    pub fn new(size: usize) -> Self {
        Self { size, data: vec![[0.0; 3]; size], input_range: (0.0, 1.0), output_range: (0.0, 1.0) }
    }
    pub fn apply(&self, value: f32) -> [f32; 3] {
        let t = (value - self.input_range.0) / (self.input_range.1 - self.input_range.0);
        let idx = (t * (self.size - 1) as f32).clamp(0.0, (self.size - 1) as f32);
        let i = idx as usize;
        let f = idx - i as f32;
        if i >= self.size - 1 { return self.data[self.size - 1]; }
        let a = self.data[i];
        let b = self.data[i + 1];
        [
            a[0] + f * (b[0] - a[0]),
            a[1] + f * (b[1] - a[1]),
            a[2] + f * (b[2] - a[2]),
        ]
    }
}

/// 3D LUT (cube)
#[derive(Debug, Clone)]
pub struct Lut3D {
    pub size: usize,
    pub data: Vec<[f32; 3]>,
    pub input_range: ([f32; 3], [f32; 3]),
    pub output_range: ([f32; 3], [f32; 3]),
}

impl Lut3D {
    pub fn new(size: usize) -> Self {
        Self { size, data: vec![[0.0; 3]; size * size * size],
            input_range: ([0.0; 3], [1.0; 3]), output_range: ([0.0; 3], [1.0; 3]) }
    }
    pub fn index(&self, r: usize, g: usize, b: usize) -> usize {
        (r * self.size + g) * self.size + b
    }
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        // Trilinear interpolation
        let s = self.size as f32 - 1.0;
        let r = (rgb[0] - self.input_range.0[0]) / (self.input_range.1[0] - self.input_range.0[0]) * s;
        let g = (rgb[1] - self.input_range.0[1]) / (self.input_range.1[1] - self.input_range.0[1]) * s;
        let b = (rgb[2] - self.input_range.0[2]) / (self.input_range.1[2] - self.input_range.0[2]) * s;

        let r0 = r.floor() as usize; let r1 = (r0 + 1).min(self.size - 1);
        let g0 = g.floor() as usize; let g1 = (g0 + 1).min(self.size - 1);
        let b0 = b.floor() as usize; let b1 = (b0 + 1).min(self.size - 1);

        let fr = r - r0 as f32;
        let fg = g - g0 as f32;
        let fb = b - b0 as f32;

        let i000 = self.index(r0, g0, b0);
        let i001 = self.index(r0, g0, b1);
        let i010 = self.index(r0, g1, b0);
        let i011 = self.index(r0, g1, b1);
        let i100 = self.index(r1, g0, b0);
        let i101 = self.index(r1, g0, b1);
        let i110 = self.index(r1, g1, b0);
        let i111 = self.index(r1, g1, b1);

        let c000 = self.data[i000];
        let c001 = self.data[i001];
        let c010 = self.data[i010];
        let c011 = self.data[i011];
        let c100 = self.data[i100];
        let c101 = self.data[i101];
        let c110 = self.data[i110];
        let c111 = self.data[i111];

        let mut out = [0.0; 3];
        for c in 0..3 {
            let c00 = c000[c] + fr * (c100[c] - c000[c]);
            let c01 = c010[c] + fr * (c110[c] - c010[c]);
            let c10 = c001[c] + fr * (c101[c] - c001[c]);
            let c11 = c011[c] + fr * (c111[c] - c011[c]);
            let c0 = c00 + fg * (c01 - c00);
            let c1 = c10 + fg * (c11 - c10);
            out[c] = c0 + fb * (c1 - c0);
        }
        out
    }
}

/// CDL (Color Decision List) - ASC standard
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cdl {
    pub slope: [f32; 3],
    pub offset: [f32; 3],
    pub power: [f32; 3],
    pub sat: f32,
}

impl Default for Cdl {
    fn default() -> Self {
        Self { slope: [1.0, 1.0, 1.0], offset: [0.0, 0.0, 0.0], power: [1.0, 1.0, 1.0], sat: 1.0 }
    }
}

impl Cdl {
    pub fn apply(self, rgb: [f32; 3]) -> [f32; 3] {
        let mut out = [
            (rgb[0] * self.slope[0] + self.offset[0]).max(0.0).powf(self.power[0]),
            (rgb[1] * self.slope[1] + self.offset[1]).max(0.0).powf(self.power[1]),
            (rgb[2] * self.slope[2] + self.offset[2]).max(0.0).powf(self.power[2]),
        ];
        // Saturation
        let luma = 0.2126 * out[0] + 0.7152 * out[1] + 0.0722 * out[2];
        out[0] = luma + self.sat * (out[0] - luma);
        out[1] = luma + self.sat * (out[1] - luma);
        out[2] = luma + self.sat * (out[2] - luma);
        out
    }
}

/// Color grade node (for node-based grading)
#[derive(Debug, Clone)]
pub struct GradeNode {
    pub id: crate::id::GradeNodeId,
    pub enabled: bool,
    pub label: String,
    pub color_space: ColorSpace,
    pub operations: Vec<GradeOp>,
    pub cache: Option<GradeCache>,
    pub position: (f32, f32), // UI position
}

#[derive(Debug, Clone)]
pub enum GradeOp {
    // Primary
    LiftGammaGain { lift: [f32; 3], gamma: [f32; 3], gain: [f32; 3] },
    OffsetContrastPivot { offset: [f32; 3], contrast: f32, pivot: f32 },
    TemperatureTint { temp: f32, tint: f32 },
    // Log wheels
    LogWheels { shadows: [f32; 3], midtones: [f32; 3], highlights: [f32; 3], range: [f32; 2] },
    // Curves
    CustomCurves { channels: [Curve; 4] }, // RGB + Luma
    HueVsHue { points: Vec<(f32, f32)> },
    HueVsSat { points: Vec<(f32, f32)> },
    HueVsLum { points: Vec<(f32, f32)> },
    SatVsSat { points: Vec<(f32, f32)> },
    LumVsSat { points: Vec<(f32, f32)> },
    // Secondary
    Qualifier { h_range: (f32, f32), s_range: (f32, f32), l_range: (f32, f32), softness: f32 },
    Window { shape: WindowShape, transform: WindowTransform },
    Tracker { data: Vec<TrackPoint> },
    // LUT
    Lut3D { lut: Lut3D, mix: f32 },
    Lut1D { lut: Lut1D, mix: f32 },
    // Creative
    FilmGrain { strength: f32, size: f32 },
    Halation { strength: f32, spread: f32 },
    Bloom { threshold: f32, strength: f32, radius: f32 },
    Vignette { amount: f32, midpoint: f32, roundness: f32, feather: f32 },
    // Technical
    ColorSpaceTransform { from: ColorSpace, to: ColorSpace },
    Cdl(Cdl),
    // Blur/Sharpen
    Blur { radius: f32, shape: BlurShape },
    Sharpen { amount: f32, radius: f32, threshold: f32 },
    NoiseReduction { spatial: f32, temporal: f32 },
    // Lens
    LensDistortion { k1: f32, k2: f32, k3: f32 },
    ChromaticAberration { amount: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlurShape {
    Gaussian,
    Box,
    Disc,
    Variable,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    pub points: Vec<(f32, f32)>, // (input, output) 0-1
    pub interpolation: CurveInterp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveInterp {
    Linear,
    Spline,
    Constant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowShape {
    Circle,
    Square,
    Rectangle,
    Ellipse,
    Polygon,
    Gradient,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowTransform {
    pub center: (f32, f32),
    pub size: (f32, f32),
    pub rotation: f32,
    pub aspect: f32,
    pub softness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackPoint {
    pub frame: crate::time::Frame,
    pub position: (f32, f32),
    pub scale: (f32, f32),
    pub rotation: f32,
}

#[derive(Debug, Clone, Default)]
pub struct GradeCache {
    pub lut3d: Option<Lut3D>,
    pub lut1d: Option<Lut1D>,
    pub invalidated: bool,
}

/// Video scopes data
#[derive(Debug, Clone, Default)]
pub struct ScopesData {
    pub waveform: Vec<f32>,     // 256 * 3 (RGB)
    pub parade: Vec<f32>,       // 256 * 3
    pub vectorscope: Vec<f32>,  // 256 * 256
    pub histogram: Vec<f32>,    // 256 * 4 (RGB + Luma)
    pub frame: crate::time::Frame,
}

impl ScopesData {
    pub fn new() -> Self {
        Self {
            waveform: vec![0.0; 256 * 3],
            parade: vec![0.0; 256 * 3],
            vectorscope: vec![0.0; 256 * 256],
            histogram: vec![0.0; 256 * 4],
            frame: crate::time::Frame::ZERO,
        }
    }
}