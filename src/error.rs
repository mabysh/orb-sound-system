use std::error::Error;
use std::fmt::{Display, Formatter};

use rodio::{DevicesError, PlayError, StreamError};

#[derive(Debug)]
pub enum OrbSoundSystemError {
    DeviceErr(DevicesError),
    StreamErr(StreamError),
    PlayErr(PlayError),
    SoundFileErr(String)
}

impl Display for OrbSoundSystemError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OrbSoundSystemError::DeviceErr(err) => Display::fmt(&err, f),
            OrbSoundSystemError::StreamErr(err) => Display::fmt(&err, f),
            OrbSoundSystemError::PlayErr(err) => Display::fmt(&err, f),
            OrbSoundSystemError::SoundFileErr(msg) => Display::fmt(&msg, f)
        }
    }
}

impl Error for OrbSoundSystemError {}
