use std::time::{Duration, Instant};

use crate::decode::Decoded;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Paused,
    Playing,
    Present,
    PresentRandom,
}

#[derive(Debug)]
pub struct Playback {
    pub state: PlaybackState,
    frame: usize,
    frame_started: Instant,
    present_interval: Duration,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            state: PlaybackState::Playing,
            frame: 0,
            frame_started: Instant::now(),
            present_interval: Duration::from_secs(5),
        }
    }
}

impl Playback {
    pub fn frame_index(&self) -> usize {
        self.frame
    }

    pub fn reset(&mut self) {
        self.frame = 0;
        self.frame_started = Instant::now();
    }

    pub fn set_state(&mut self, state: PlaybackState) {
        self.state = state;
        self.frame_started = Instant::now();
    }

    pub fn toggle_play(&mut self) {
        self.state = match self.state {
            PlaybackState::Playing => PlaybackState::Paused,
            _ => PlaybackState::Playing,
        };
        self.frame_started = Instant::now();
    }

    pub fn set_present_interval(&mut self, interval: Duration) {
        self.present_interval = interval;
    }

    pub fn frame_delay(decoded: &Decoded, frame: usize) -> Option<Duration> {
        match decoded {
            Decoded::Still(_) => None,
            Decoded::Animation(frames) => frames
                .get(frame)
                .map(|f| if f.delay.is_zero() { Duration::from_millis(100) } else { f.delay }),
        }
    }

    pub fn advance(&mut self, decoded: &Decoded, now: Instant) -> bool {
        if self.state != PlaybackState::Playing {
            return false;
        }
        let Decoded::Animation(frames) = decoded else {
            return false;
        };
        if frames.len() < 2 {
            return false;
        }

        let mut advanced = false;
        while let Some(delay) = Self::frame_delay(decoded, self.frame) {
            if now.duration_since(self.frame_started) < delay {
                break;
            }
            self.frame = (self.frame + 1) % frames.len();
            self.frame_started += delay;
            advanced = true;
        }
        advanced
    }

    pub fn present_due(&mut self, now: Instant) -> bool {
        if !matches!(self.state, PlaybackState::Present | PlaybackState::PresentRandom) {
            return false;
        }
        if now.duration_since(self.frame_started) >= self.present_interval {
            self.frame_started = now;
            return true;
        }
        false
    }
}
