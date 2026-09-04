use std::time::{Duration, Instant};

use reveal::decode::{Decoded, DecodedImage, Frame};
use reveal::playback::{Playback, PlaybackState};

fn anim(delays_ms: &[u64]) -> Decoded {
    Decoded::Animation(
        delays_ms
            .iter()
            .map(|ms| Frame {
                image: DecodedImage { rgba: vec![0, 0, 0, 255], width: 1, height: 1 },
                delay: Duration::from_millis(*ms),
            })
            .collect(),
    )
}

fn still() -> Decoded {
    Decoded::Still(DecodedImage { rgba: vec![0, 0, 0, 255], width: 1, height: 1 })
}

#[test]
fn honours_per_frame_delays_not_a_fixed_tick() {
    let decoded = anim(&[500, 50, 50]);
    let mut p = Playback::default();
    let t0 = Instant::now();

    p.advance(&decoded, t0 + Duration::from_millis(100));
    assert_eq!(p.frame_index(), 0, "long first frame must still be showing");

    p.advance(&decoded, t0 + Duration::from_millis(510));
    assert_eq!(p.frame_index(), 1, "advances once its own 500ms elapsed");

    p.advance(&decoded, t0 + Duration::from_millis(565));
    assert_eq!(p.frame_index(), 2, "short frame advances quickly");
}

#[test]
fn catches_up_across_multiple_frames_after_a_stall() {
    let decoded = anim(&[10, 10, 10, 10]);
    let mut p = Playback::default();
    let t0 = Instant::now();

    p.advance(&decoded, t0 + Duration::from_millis(35));
    assert_eq!(p.frame_index(), 3, "a long stall should not drop the clock");
}

#[test]
fn wraps_to_the_first_frame() {
    let decoded = anim(&[10, 10]);
    let mut p = Playback::default();
    let t0 = Instant::now();
    p.advance(&decoded, t0 + Duration::from_millis(25));
    assert_eq!(p.frame_index(), 0);
}

#[test]
fn paused_animation_does_not_advance() {
    let decoded = anim(&[10, 10]);
    let mut p = Playback::default();
    p.set_state(PlaybackState::Paused);
    assert!(!p.advance(&decoded, Instant::now() + Duration::from_secs(5)));
    assert_eq!(p.frame_index(), 0);
}

#[test]
fn stills_never_advance() {
    let decoded = still();
    let mut p = Playback::default();
    assert!(!p.advance(&decoded, Instant::now() + Duration::from_secs(60)));
}

#[test]
fn zero_delay_frames_fall_back_to_a_sane_rate() {
    let decoded = anim(&[0, 0]);
    let delay = Playback::frame_delay(&decoded, 0).unwrap();
    assert_eq!(delay, Duration::from_millis(100));
}

#[test]
fn presentation_advances_on_its_interval() {
    let mut p = Playback::default();
    p.set_present_interval(Duration::from_millis(200));
    p.set_state(PlaybackState::Present);
    let t0 = Instant::now();

    assert!(!p.present_due(t0 + Duration::from_millis(100)));
    assert!(p.present_due(t0 + Duration::from_millis(250)));
}

#[test]
fn presentation_is_inert_while_merely_playing() {
    let mut p = Playback::default();
    p.set_state(PlaybackState::Playing);
    assert!(!p.present_due(Instant::now() + Duration::from_secs(60)));
}
