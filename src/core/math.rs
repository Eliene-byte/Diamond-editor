//! Math utilities for video editing
//! SIMD-optimized where possible

use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
use std::ops::{Add, Div, Mul, Sub};

/// 2D point with frame precision
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point2 {
    pub x: f32,
    pub y: f32,
}

impl Point2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn from_vec2(v: Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

impl Add for Point2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Point2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul<f32> for Point2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Div<f32> for Point2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

/// Rectangle with float coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub const fn from_points(min: Point2, max: Point2) -> Self {
        Self {
            x: min.x,
            y: min.y,
            w: max.x - min.x,
            h: max.y - min.y,
        }
    }
    pub const fn contains(self, p: Point2) -> bool {
        p.x >= self.x && p.x < self.x + self.w && p.y >= self.y && p.y < self.y + self.h
    }
    pub const fn center(self) -> Point2 {
        Point2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
    pub fn intersect(self, other: Self) -> Option<Self> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.w).min(other.x + other.w);
        let y2 = (self.y + self.h).min(other.y + other.h);
        if x1 < x2 && y1 < y2 {
            Some(Self::new(x1, y1, x2 - x1, y2 - y1))
        } else {
            None
        }
    }
    pub fn union(self, other: Self) -> Self {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.w).max(other.x + other.w);
        let y2 = (self.y + self.h).max(other.y + other.h);
        Self::new(x1, y1, x2 - x1, y2 - y1)
    }
    pub fn inflate(self, dx: f32, dy: f32) -> Self {
        Self::new(
            self.x - dx,
            self.y - dy,
            self.w + 2.0 * dx,
            self.h + 2.0 * dy,
        )
    }
}

/// Transform matrix for 2D (3x3 homogeneous)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub m: [[f32; 3]; 3],
}

impl Transform2D {
    pub const IDENTITY: Self = Self {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    pub fn translate(x: f32, y: f32) -> Self {
        Self {
            m: [[1.0, 0.0, x], [0.0, 1.0, y], [0.0, 0.0, 1.0]],
        }
    }
    pub fn scale(x: f32, y: f32) -> Self {
        Self {
            m: [[x, 0.0, 0.0], [0.0, y, 0.0], [0.0, 0.0, 1.0]],
        }
    }
    pub fn rotate(angle: f32) -> Self {
        let c = angle.cos();
        let s = angle.sin();
        Self {
            m: [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
        }
    }
    pub fn shear(x: f32, y: f32) -> Self {
        Self {
            m: [[1.0, x, 0.0], [y, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    pub fn multiply(self, other: Self) -> Self {
        let mut m = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] = self.m[i][0] * other.m[0][j]
                    + self.m[i][1] * other.m[1][j]
                    + self.m[i][2] * other.m[2][j];
            }
        }
        Self { m }
    }

    pub fn transform_point(self, p: Point2) -> Point2 {
        let x = self.m[0][0] * p.x + self.m[0][1] * p.y + self.m[0][2];
        let y = self.m[1][0] * p.x + self.m[1][1] * p.y + self.m[1][2];
        let w = self.m[2][0] * p.x + self.m[2][1] * p.y + self.m[2][2];
        if w != 0.0 && w != 1.0 {
            Point2::new(x / w, y / w)
        } else {
            Point2::new(x, y)
        }
    }

    pub fn transform_vector(self, v: Vec2) -> Vec2 {
        Vec2::new(
            self.m[0][0] * v.x + self.m[0][1] * v.y,
            self.m[1][0] * v.x + self.m[1][1] * v.y,
        )
    }

    pub fn inverse(self) -> Option<Self> {
        let det = self.m[0][0] * (self.m[1][1] * self.m[2][2] - self.m[1][2] * self.m[2][1])
            - self.m[0][1] * (self.m[1][0] * self.m[2][2] - self.m[1][2] * self.m[2][0])
            + self.m[0][2] * (self.m[1][0] * self.m[2][1] - self.m[1][1] * self.m[2][0]);
        if det.abs() < 1e-6 {
            return None;
        }
        let inv_det = 1.0 / det;
        let mut m = [[0.0; 3]; 3];
        m[0][0] = (self.m[1][1] * self.m[2][2] - self.m[1][2] * self.m[2][1]) * inv_det;
        m[0][1] = (self.m[0][2] * self.m[2][1] - self.m[0][1] * self.m[2][2]) * inv_det;
        m[0][2] = (self.m[0][1] * self.m[1][2] - self.m[0][2] * self.m[1][1]) * inv_det;
        m[1][0] = (self.m[1][2] * self.m[2][0] - self.m[1][0] * self.m[2][2]) * inv_det;
        m[1][1] = (self.m[0][0] * self.m[2][2] - self.m[0][2] * self.m[2][0]) * inv_det;
        m[1][2] = (self.m[0][2] * self.m[1][0] - self.m[0][0] * self.m[1][2]) * inv_det;
        m[2][0] = (self.m[1][0] * self.m[2][1] - self.m[1][1] * self.m[2][0]) * inv_det;
        m[2][1] = (self.m[0][1] * self.m[2][0] - self.m[0][0] * self.m[2][1]) * inv_det;
        m[2][2] = (self.m[0][0] * self.m[1][1] - self.m[0][1] * self.m[1][0]) * inv_det;
        Some(Self { m })
    }

    pub fn decompose(self) -> Option<(Vec2, f32, Vec2)> {
        // Extract translation, rotation, scale
        let tx = self.m[0][2];
        let ty = self.m[1][2];
        let sx = (self.m[0][0] * self.m[0][0] + self.m[1][0] * self.m[1][0]).sqrt();
        let sy = (self.m[0][1] * self.m[0][1] + self.m[1][1] * self.m[1][1]).sqrt();
        if sx < 1e-6 || sy < 1e-6 {
            return None;
        }
        let rot = self.m[1][0].atan2(self.m[0][0]);
        Some((Vec2::new(tx, ty), rot, Vec2::new(sx, sy)))
    }
}

/// Easing functions for animation
pub mod easing {
    pub fn linear(t: f32) -> f32 {
        t
    }
    pub fn ease_in_quad(t: f32) -> f32 {
        t * t
    }
    pub fn ease_out_quad(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t)
    }
    pub fn ease_in_out_quad(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - 2.0 * (1.0 - t) * (1.0 - t)
        }
    }
    pub fn ease_in_cubic(t: f32) -> f32 {
        t * t * t
    }
    pub fn ease_out_cubic(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(3)
    }
    pub fn ease_in_out_cubic(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }
    pub fn ease_in_quart(t: f32) -> f32 {
        t * t * t * t
    }
    pub fn ease_out_quart(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(4)
    }
    pub fn ease_in_out_quart(t: f32) -> f32 {
        if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
        }
    }
    pub fn ease_in_expo(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else {
            2.0_f32.powf(10.0 * (t - 1.0))
        }
    }
    pub fn ease_out_expo(t: f32) -> f32 {
        if t == 1.0 {
            1.0
        } else {
            1.0 - 2.0_f32.powf(-10.0 * t)
        }
    }
    pub fn ease_in_out_expo(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else if t < 0.5 {
            2.0_f32.powf(20.0 * t - 10.0) / 2.0
        } else {
            (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
        }
    }
    pub fn ease_in_back(t: f32) -> f32 {
        let c = 1.70158;
        (c + 1.0) * t * t * t - c * t * t
    }
    pub fn ease_out_back(t: f32) -> f32 {
        let c = 1.70158;
        1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
    }
    pub fn ease_in_out_back(t: f32) -> f32 {
        let c = 1.70158 * 1.525;
        if t < 0.5 {
            (2.0 * t).powi(2) * ((c + 1.0) * 2.0 * t - c) / 2.0
        } else {
            ((2.0 * t - 2.0).powi(2) * ((c + 1.0) * (2.0 * t - 2.0) + c) + 2.0) / 2.0
        }
    }
    pub fn ease_out_elastic(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            2.0_f32.powf(-10.0 * t) * (t * 10.0 - 0.75) * (2.0 * std::f32::consts::PI / 3.0).sin()
                + 1.0
        }
    }
    pub fn ease_out_bounce(t: f32) -> f32 {
        if t < 1.0 / 2.75 {
            7.5625 * t * t
        } else if t < 2.0 / 2.75 {
            let t = t - 1.5 / 2.75;
            7.5625 * t * t + 0.75
        } else if t < 2.5 / 2.75 {
            let t = t - 2.25 / 2.75;
            7.5625 * t * t + 0.9375
        } else {
            let t = t - 2.625 / 2.75;
            7.5625 * t * t + 0.984375
        }
    }
    pub fn cubic_bezier(t: f32, p1: f32, p2: f32, p3: f32, p4: f32) -> f32 {
        let u = 1.0 - t;
        u * u * u * 0.0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * 1.0
    }
}

/// Interpolation for keyframes
pub mod interp {
    use super::easing;
    use crate::core::types::{Easing, Interpolation};

    pub fn interpolate(a: f32, b: f32, t: f32, interp: Interpolation, easing: Easing) -> f32 {
        let t = match interp {
            Interpolation::Linear => t,
            Interpolation::Constant => 0.0,
            Interpolation::Hold => 0.0,
            Interpolation::Bezier => t, // Handled separately with control points
        };
        let t = apply_easing(t, easing);
        a + (b - a) * t
    }

    pub fn apply_easing(t: f32, easing: Easing) -> f32 {
        match easing {
            Easing::None => easing::linear(t),
            Easing::EaseIn => easing::ease_in_cubic(t),
            Easing::EaseOut => easing::ease_out_cubic(t),
            Easing::EaseInOut => easing::ease_in_out_cubic(t),
            Easing::Custom(x1, y1, x2, y2) => easing::cubic_bezier(t, x1, y1, x2, y2),
        }
    }

    pub fn interpolate_vec3(
        a: glam::Vec3,
        b: glam::Vec3,
        t: f32,
        interp: Interpolation,
        easing: Easing,
    ) -> glam::Vec3 {
        let t = apply_easing(t, easing);
        a.lerp(b, t)
    }

    pub fn interpolate_color(
        a: [f32; 3],
        b: [f32; 3],
        t: f32,
        interp: Interpolation,
        easing: Easing,
    ) -> [f32; 3] {
        let t = apply_easing(t, easing);
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]
    }
}

/// Bounding box hierarchy for spatial queries
#[derive(Debug, Clone)]
pub struct Bvh2D<T> {
    nodes: Vec<BvhNode<T>>,
}

#[derive(Debug, Clone)]
struct BvhNode<T> {
    bounds: Rect,
    left: Option<usize>,
    right: Option<usize>,
    data: Option<T>,
}

impl<T: Clone> Bvh2D<T> {
    pub fn new(items: Vec<(Rect, T)>) -> Self {
        let mut nodes = Vec::new();
        let root = Self::build(&mut nodes, items);
        Self { nodes }
    }

    fn build(nodes: &mut Vec<BvhNode<T>>, mut items: Vec<(Rect, T)>) -> usize {
        let idx = nodes.len();
        nodes.push(BvhNode {
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            left: None,
            right: None,
            data: None,
        });
        if items.len() == 1 {
            nodes[idx].bounds = items[0].0;
            nodes[idx].data = Some(items[0].1.clone());
            return idx;
        }
        // Find best split axis
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for (r, _) in &items {
            min_x = min_x.min(r.x);
            min_y = min_y.min(r.y);
            max_x = max_x.max(r.x + r.w);
            max_y = max_y.max(r.y + r.h);
        }
        let split_x = (max_x - min_x) > (max_y - min_y);
        items.sort_by(|a, b| {
            let ca = if split_x {
                a.0.x + a.0.w * 0.5
            } else {
                a.0.y + a.0.h * 0.5
            };
            let cb = if split_x {
                b.0.x + b.0.w * 0.5
            } else {
                b.0.y + b.0.h * 0.5
            };
            ca.partial_cmp(&cb).unwrap()
        });
        let mid = items.len() / 2;
        let left = Self::build(nodes, items[..mid].to_vec());
        let right = Self::build(nodes, items[mid..].to_vec());
        nodes[idx].left = Some(left);
        nodes[idx].right = Some(right);
        nodes[idx].bounds = nodes[left].bounds.union(nodes[right].bounds);
        idx
    }

    pub fn query<'a>(&'a self, rect: Rect, results: &mut Vec<&'a T>) {
        self.query_node(0, rect, results);
    }

    fn query_node<'a>(&'a self, idx: usize, rect: Rect, results: &mut Vec<&'a T>) {
        let node = &self.nodes[idx];
        if !node.bounds.intersect(rect).is_some() {
            return;
        }
        if let Some(data) = &node.data {
            results.push(data);
        }
        if let Some(left) = node.left {
            self.query_node(left, rect, results);
        }
        if let Some(right) = node.right {
            self.query_node(right, rect, results);
        }
    }
}

/// Fast random for procedural generation
pub struct FastRand {
    state: u64,
}

impl FastRand {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn next_f32_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
    pub fn next_i32_range(&mut self, min: i32, max: i32) -> i32 {
        min + (self.next_u64() as u32 % (max - min + 1) as u32) as i32
    }
}

/// SIMD-accelerated image operations
#[cfg(target_arch = "x86_64")]
pub mod simd {
    #[target_feature(enable = "avx2")]
    pub unsafe fn blend_normal_avx2(dst: &mut [f32], src: &[f32], alpha: &[f32], len: usize) {
        use std::arch::x86_64::*;
        let mut i = 0;
        while i + 8 <= len {
            let d = _mm256_loadu_ps(dst.as_ptr().add(i));
            let s = _mm256_loadu_ps(src.as_ptr().add(i));
            let a = _mm256_loadu_ps(alpha.as_ptr().add(i));
            let one_minus_a = _mm256_sub_ps(_mm256_set1_ps(1.0), a);
            let blended = _mm256_add_ps(_mm256_mul_ps(s, a), _mm256_mul_ps(d, one_minus_a));
            _mm256_storeu_ps(dst.as_mut_ptr().add(i), blended);
            i += 8;
        }
        while i < len {
            let a = alpha[i];
            dst[i] = src[i] * a + dst[i] * (1.0 - a);
            i += 1;
        }
    }

    /// Adjust saturation on an interleaved RGB buffer.
    ///
    /// Works on 3-float groups; `len` is the number of floats, so it should be a
    /// multiple of 3. Any trailing floats that do not form a complete pixel are
    /// left untouched.
    pub fn add_saturation_avx2(dst: &mut [f32], amount: f32, len: usize) {
        // Rec.709 luma coefficients.
        const R_COEFF: f32 = 0.2126;
        const G_COEFF: f32 = 0.7152;
        const B_COEFF: f32 = 0.0722;

        let pixels = (len.min(dst.len()) / 3) * 3;
        for px in dst[..pixels].chunks_exact_mut(3) {
            let gray = px[0] * R_COEFF + px[1] * G_COEFF + px[2] * B_COEFF;
            px[0] = gray + (px[0] - gray) * amount;
            px[1] = gray + (px[1] - gray) * amount;
            px[2] = gray + (px[2] - gray) * amount;
        }
    }

    /// Runtime check for the AVX2 feature set.
    pub fn has_avx2() -> bool {
        std::arch::is_x86_feature_detected!("avx2")
    }
}

/// Portable equivalents of the SIMD helpers, so callers outside the x86_64
/// module still have something to call.
pub mod simd_fallback {
    /// Overwrite `dst` with `dst * alpha + src * (1 - alpha)` in place.
    pub fn blend_normal(dst: &mut [f32], src: &[f32], alpha: &[f32], len: usize) {
        for i in 0..len {
            if i >= src.len() || i >= alpha.len() {
                break;
            }
            dst[i] = src[i] * alpha[i] + dst[i] * (1.0 - alpha[i]);
        }
    }

    /// Adjust saturation on an interleaved RGB buffer.
    pub fn add_saturation(dst: &mut [f32], amount: f32, len: usize) {
        let pixels = (len.min(dst.len()) / 3) * 3;
        for px in dst[..pixels].chunks_exact_mut(3) {
            let gray = px[0] * 0.2126 + px[1] * 0.7152 + px[2] * 0.0722;
            px[0] = gray + (px[0] - gray) * amount;
            px[1] = gray + (px[1] - gray) * amount;
            px[2] = gray + (px[2] - gray) * amount;
        }
    }
}
