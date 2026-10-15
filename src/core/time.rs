//! Frame-accurate time representation
//! Critical for professional video editing

use std::ops::{Add, Sub, AddAssign, SubAssign, Mul, Div, Rem, Neg};
use std::fmt;
use crate::FrameRate;

/// Frame count - the fundamental unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Frame(pub i64);

impl Frame {
    pub const ZERO: Self = Self(0);
    pub const MIN: Self = Self(i64::MIN);
    pub const MAX: Self = Self(i64::MAX);

    pub const fn new(frames: i64) -> Self { Self(frames) }
    pub const fn from_seconds(seconds: f64, fps: FrameRate) -> Self {
        Self((seconds * fps.fps_f32() as f64).round() as i64)
    }
    pub const fn from_milliseconds(ms: i64, fps: FrameRate) -> Self {
        let r = fps.to_rational();
        Self((ms as i128 * r.num as i128 / 1000 / r.den as i128) as i64)
    }
    pub const fn to_seconds(self, fps: FrameRate) -> f64 {
        self.0 as f64 / fps.fps_f32() as f64
    }
    pub const fn to_milliseconds(self, fps: FrameRate) -> i64 {
        let r = fps.to_rational();
        (self.0 as i128 * 1000 * r.den as i128 / r.num as i128) as i64
    }
    pub const fn abs(self) -> Self { Self(self.0.abs()) }
    pub const fn signum(self) -> Self { Self(self.0.signum()) }
    pub const fn is_negative(self) -> bool { self.0 < 0 }
    pub const fn is_zero(self) -> bool { self.0 == 0 }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}f", self.0)
    }
}

impl Add for Frame {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output { Self(self.0 + rhs.0) }
}

impl Sub for Frame {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output { Self(self.0 - rhs.0) }
}

impl AddAssign for Frame {
    fn add_assign(&mut self, rhs: Self) { self.0 += rhs.0; }
}

impl SubAssign for Frame {
    fn sub_assign(&mut self, rhs: Self) { self.0 -= rhs.0; }
}

impl Mul<i64> for Frame {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self::Output { Self(self.0 * rhs) }
}

impl Div<i64> for Frame {
    type Output = Self;
    fn div(self, rhs: i64) -> Self::Output { Self(self.0 / rhs) }
}

impl Rem<i64> for Frame {
    type Output = Self;
    fn rem(self, rhs: i64) -> Self::Output { Self(self.0 % rhs) }
}

impl Neg for Frame {
    type Output = Self;
    fn neg(self) -> Self::Output { Self(-self.0) }
}

/// Rational number for exact frame rates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

impl Rational {
    pub const fn new(num: i64, den: i64) -> Self {
        Self { num, den }
    }
    pub const fn from_fps(fps: f64) -> Self {
        // Approximate - not exact
        let num = (fps * 1000.0).round() as i64;
        Self { num, den: 1000 }
    }
    pub const fn reduce(self) -> Self {
        // Simple GCD reduction
        fn gcd(mut a: i64, mut b: i64) -> i64 {
            while b != 0 { let t = b; b = a % b; a = t; } a.abs()
        }
        let g = gcd(self.num, self.den);
        Self { num: self.num / g, den: self.den / g }
    }
    pub const fn as_f32(self) -> f32 { self.num as f32 / self.den as f32 }
    pub const fn as_f64(self) -> f64 { self.num as f64 / self.den as f64 }
    pub const fn inverted(self) -> Self { Self { num: self.den, den: self.num } }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

/// SMPTE Timecode
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timecode {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
    pub drop_frame: bool,
    pub fps: FrameRate,
}

impl Timecode {
    pub const fn zero(fps: FrameRate) -> Self {
        Self { hours: 0, minutes: 0, seconds: 0, frames: 0, drop_frame: false, fps }
    }

    pub const fn new(h: u8, m: u8, s: u8, f: u8, drop: bool, fps: FrameRate) -> Self {
        Self { hours: h, minutes: m, seconds: s, frames: f, drop_frame: drop, fps }
    }

    pub fn from_frame(frame: Frame, fps: FrameRate) -> Self {
        let total = frame.0;
        let r = fps.to_rational();
        let drop = matches!(fps, FrameRate::Fps29_97 | FrameRate::Fps59_94);

        let frames_per_day = 24 * 60 * 60 * r.num / r.den;
        let frames_per_hour = 60 * 60 * r.num / r.den;
        let frames_per_min = 60 * r.num / r.den;
        let frames_per_sec = r.num / r.den;

        let mut total = total.rem_euclid(frames_per_day);
        if total < 0 { total += frames_per_day; }

        let hours = (total / frames_per_hour) as u8;
        total %= frames_per_hour;
        let minutes = (total / frames_per_min) as u8;
        total %= frames_per_min;

        if drop {
            // Drop frame calculation
            let frames_per_10min = frames_per_min * 10 - 2 * (r.num / r.den) as i64;
            let ten_min_chunks = total / frames_per_10min;
            total %= frames_per_10min;
            let frames_per_min_df = frames_per_min - 2 * (r.num / r.den) as i64;
            let mut minutes_calc = (total / frames_per_min_df) as u8;
            if minutes_calc >= 10 { minutes_calc = 9; }
            total %= frames_per_min_df;

            let seconds = (total / frames_per_sec) as u8;
            total %= frames_per_sec;
            let frames = total as u8;

            Self { hours, minutes: minutes_calc + ten_min_chunks * 10, seconds, frames, drop_frame: true, fps }
        } else {
            let seconds = (total / frames_per_sec) as u8;
            total %= frames_per_sec;
            let frames = total as u8;
            Self { hours, minutes, seconds, frames, drop_frame: false, fps }
        }
    }

    pub fn to_frame(self) -> Frame {
        let r = self.fps.to_rational();
        let frames_per_hour = 60 * 60 * r.num / r.den;
        let frames_per_min = 60 * r.num / r.den;
        let frames_per_sec = r.num / r.den;

        let mut total = self.hours as i64 * frames_per_hour
            + self.minutes as i64 * frames_per_min
            + self.seconds as i64 * frames_per_sec
            + self.frames as i64;

        if self.drop_frame {
            // Add back dropped frames
            let drop_frames = 2 * (r.num / r.den) as i64;
            let total_mins = self.hours as i64 * 60 + self.minutes as i64;
            let ten_min_chunks = total_mins / 10;
            let remaining_mins = total_mins % 10;
            total += ten_min_chunks * drop_frames;
            if remaining_mins > 0 {
                total += drop_frames * ((remaining_mins - 1) as i64);
            }
        }

        Frame(total)
    }

    pub fn format(&self) -> String {
        let sep = if self.drop_frame { ";" } else { ":" };
        format!("{:02}{}{:02}{}{:02}{}{:02}",
            self.hours, sep, self.minutes, sep, self.seconds, sep, self.frames)
    }

    pub fn parse(s: &str, fps: FrameRate) -> Option<Self> {
        let drop = s.contains(';');
        let parts: Vec<&str> = s.split(&[':', ';'][..]).collect();
        if parts.len() != 4 { return None; }
        Some(Self {
            hours: parts[0].parse().ok()?,
            minutes: parts[1].parse().ok()?,
            seconds: parts[2].parse().ok()?,
            frames: parts[3].parse().ok()?,
            drop_frame: drop,
            fps,
        })
    }
}

impl fmt::Display for Timecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Time duration with frame accuracy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
    pub frames: Frame,
    pub fps: FrameRate,
}

impl Duration {
    pub const fn new(frames: Frame, fps: FrameRate) -> Self { Self { frames, fps } }
    pub const fn zero(fps: FrameRate) -> Self { Self { frames: Frame::ZERO, fps } }
    pub const fn from_seconds(seconds: f64, fps: FrameRate) -> Self {
        Self { frames: Frame::from_seconds(seconds, fps), fps }
    }
    pub const fn from_milliseconds(ms: i64, fps: FrameRate) -> Self {
        Self { frames: Frame::from_milliseconds(ms, fps), fps }
    }
    pub const fn frames(self) -> Frame { self.frames }
    pub const fn seconds(self) -> f64 { self.frames.to_seconds(self.fps) }
    pub const fn milliseconds(self) -> i64 { self.frames.to_milliseconds(self.fps) }
}

impl Add for Duration {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        assert_eq!(self.fps, rhs.fps);
        Self { frames: self.frames + rhs.frames, fps: self.fps }
    }
}

impl Sub for Duration {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        assert_eq!(self.fps, rhs.fps);
        Self { frames: self.frames - rhs.frames, fps: self.fps }
    }
}

/// Frame range [start, end) - half-open interval
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRange {
    pub start: Frame,
    pub end: Frame,
}

impl FrameRange {
    pub fn new(start: Frame, end: Frame) -> Self {
        assert!(start <= end, "start must be <= end");
        Self { start, end }
    }
    pub const fn empty(at: Frame) -> Self { Self { start: at, end: at } }
    pub const fn length(self) -> Frame { Frame(self.end.0 - self.start.0) }
    pub const fn contains(self, frame: Frame) -> bool { self.start <= frame && frame < self.end }
    pub const fn intersects(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
    pub fn union(self, other: Self) -> Self {
        Self { start: self.start.min(other.start), end: self.end.max(other.end) }
    }
    pub fn intersection(self, other: Self) -> Option<Self> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end { Some(Self { start, end }) } else { None }
    }
    pub fn offset(self, delta: Frame) -> Self {
        Self { start: self.start + delta, end: self.end + delta }
    }
}

/// Playback range with loop points
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackRange {
    pub range: FrameRange,
    pub loop_start: Option<Frame>,
    pub loop_end: Option<Frame>,
}

/// Time position in timeline (can be between frames for sub-frame accuracy)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct TimelinePosition {
    pub frame: Frame,
    pub subframe: f32, // 0.0 - 1.0
}

impl TimelinePosition {
    pub const fn new(frame: Frame) -> Self { Self { frame, subframe: 0.0 } }
    pub const fn from_fraction(frame: Frame, subframe: f32) -> Self {
        assert!(subframe >= 0.0 && subframe < 1.0);
        Self { frame, subframe }
    }
    pub const fn to_f32(self, fps: FrameRate) -> f32 {
        self.frame.0 as f32 + self.subframe
    }
}