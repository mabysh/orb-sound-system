//! Sound subsystem library for Worldcoin Orb device.
//!
//! Provides possibility to;
//!
//! - Play WAV files from file system
//! - Control volume by setting exact value or adjusting by given amount
//! - Pause/Resume playback
//!
//! Under the hood it runs event loop on a separate thread and uses ring buffer to eliminate buffer
//! under-run conditions. Basic usage:
//!
//! ```no_run
//! use std::time::Duration;
//! use orb_sound::handle::SoundPriority;
//! use orb_sound::OrbSoundSystem;
//!
//! // Retrieve a handle to sound system. This handle can
//! // be safely cloned and moved between threads
//! let mut sound_system_handle = OrbSoundSystem::run().unwrap();
//! // Play sound
//! sound_system_handle.play_sound(
//!     "path/to/sound.wav",            // path to WAV file
//!      SoundPriority::High,           // Sound priority
//!      Some(Duration::from_secs(2))   // Max time window sound should be played
//! ).unwrap();
//! ```
use rodio::{DevicesError, PlayError, StreamError};
use thiserror_impl::Error;

pub use handle::OrbSoundSystemHandle;
pub use system::OrbSoundSystem;

pub mod handle;
pub mod system;

#[derive(Error, Debug)]
pub enum OrbSoundSystemError {
    #[error("Sound device error")]
    DeviceErr(DevicesError),
    #[error("Sound stream error")]
    StreamErr(StreamError),
    #[error("Playback error")]
    PlayErr(PlayError),
    #[error("Sound file error")]
    SoundFileErr(String),
    #[error("System is down")]
    SystemIsDown,
}
