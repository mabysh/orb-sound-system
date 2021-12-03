use rodio::{DevicesError, PlayError, StreamError};
use thiserror_impl::Error;

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
