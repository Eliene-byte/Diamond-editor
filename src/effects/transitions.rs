//! Transition definitions for effects layer.
//!
//! The authoritative transition types (TransitionType, TransitionAlignment,
//! Transition) live in `crate::timeline::transitions`, since transitions are a
//! timeline editing primitive. This module re-exports them and adds the
//! effect-side helpers used by the inspector, so effects code does not need to
//! reach into the timeline module for the enum values.

use crate::core::id::TransitionId;
use crate::core::time::{Frame, FrameRate};
pub use crate::timeline::transitions::{Transition, TransitionAlignment, TransitionType};

/// Default transition duration in frames at the given frame rate.
pub fn default_transition_duration(fps: FrameRate) -> Frame {
    let r = fps.to_rational();
    Frame(r.num / 2) // 0.5 seconds
}

/// Minimum transition duration in frames.
pub fn min_transition_duration() -> Frame {
    Frame(1)
}

/// Maximum transition duration in frames (2 seconds at 24fps).
pub fn max_transition_duration() -> Frame {
    Frame(48)
}

/// All transition types offered by the Edit page UI, in menu order.
pub const TRANSITION_PALETTE: &[TransitionType] = &[
    // Dissolves
    TransitionType::CrossDissolve,
    TransitionType::FilmDissolve,
    TransitionType::AdditiveDissolve,
    TransitionType::MorphCut,
    TransitionType::SmoothCut,
    // Wipes
    TransitionType::WipeLeft,
    TransitionType::WipeRight,
    TransitionType::WipeUp,
    TransitionType::WipeDown,
    TransitionType::WipeCenter,
    TransitionType::WipeClock,
    TransitionType::WipeCounterClock,
    // Slides
    TransitionType::SlideLeft,
    TransitionType::SlideRight,
    TransitionType::SlideUp,
    TransitionType::SlideDown,
    // Pushes
    TransitionType::PushLeft,
    TransitionType::PushRight,
    TransitionType::PushUp,
    TransitionType::PushDown,
    // 3D / special
    TransitionType::CubeSpin,
    TransitionType::FlipOver,
    TransitionType::PagePeel,
    TransitionType::DipToColor,
    // Audio
    TransitionType::CrossFade,
    TransitionType::ConstantPower,
    TransitionType::ConstantGain,
];

/// Human-readable label for a transition type, as shown in the UI.
pub fn transition_label(transition_type: TransitionType) -> &'static str {
    use TransitionType::*;
    match transition_type {
        CrossDissolve => "Cross Dissolve",
        FilmDissolve => "Film Dissolve",
        AdditiveDissolve => "Additive Dissolve",
        WipeLeft => "Wipe Left",
        WipeRight => "Wipe Right",
        WipeUp => "Wipe Up",
        WipeDown => "Wipe Down",
        WipeCenter => "Wipe Center",
        WipeClock => "Wipe Clock",
        WipeCounterClock => "Wipe Counter Clockwise",
        SlideLeft => "Slide Left",
        SlideRight => "Slide Right",
        SlideUp => "Slide Up",
        SlideDown => "Slide Down",
        PushLeft => "Push Left",
        PushRight => "Push Right",
        PushUp => "Push Up",
        PushDown => "Push Down",
        CubeSpin => "Cube Spin",
        FlipOver => "Flip Over",
        PagePeel => "Page Peel",
        DipToColor => "Dip to Color",
        MorphCut => "Morph Cut",
        SmoothCut => "Smooth Cut",
        CrossFade => "Cross Fade",
        ConstantPower => "Constant Power",
        ConstantGain => "Constant Gain",
    }
}

/// Whether this transition type operates on audio rather than video.
pub fn is_audio_transition(transition_type: TransitionType) -> bool {
    use TransitionType::*;
    matches!(transition_type, CrossFade | ConstantPower | ConstantGain)
}

/// Whether this transition type operates on video.
pub fn is_video_transition(transition_type: TransitionType) -> bool {
    !is_audio_transition(transition_type)
}

/// Clamp a requested transition duration into the legal range.
pub fn clamp_transition_duration(duration: Frame) -> Frame {
    Frame(duration.0.clamp(min_transition_duration().0, max_transition_duration().0))
}

/// Build a transition between two timeline items with sane defaults.
pub fn make_transition(
    transition_type: TransitionType,
    left_item: crate::core::id::TrackItemId,
    right_item: crate::core::id::TrackItemId,
    fps: FrameRate,
) -> Transition {
    let duration = default_transition_duration(fps);
    let mut transition = Transition::new(transition_type, left_item, right_item, duration);
    transition.alignment = TransitionAlignment::CenterAtCut;
    transition
}

/// The transition id of an existing transition, for UI bookkeeping.
pub fn transition_id(transition: &Transition) -> TransitionId {
    transition.id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_transition_has_a_label() {
        for t in TRANSITION_PALETTE {
            assert!(!transition_label(*t).is_empty(), "missing label for {t:?}");
        }
    }

    #[test]
    fn audio_and_video_transitions_are_disjoint() {
        for t in TRANSITION_PALETTE {
            assert!(
                !(is_audio_transition(*t) && is_video_transition(*t)),
                "{t:?} classified as both audio and video"
            );
        }
    }

    #[test]
    fn duration_is_clamped_into_range() {
        let min = min_transition_duration();
        let max = max_transition_duration();
        assert_eq!(clamp_transition_duration(Frame(-10)), min);
        assert_eq!(clamp_transition_duration(Frame(100_000)), max);
        assert_eq!(clamp_transition_duration(Frame(12)), Frame(12));
    }

    #[test]
    fn default_duration_is_frame_rate_aware() {
        // 0.5s at 24fps is 12 frames; at 60fps it is 30 frames.
        assert_eq!(default_transition_duration(FrameRate::Fps24), Frame(12));
        assert_eq!(default_transition_duration(FrameRate::Fps60), Frame(30));
    }
}