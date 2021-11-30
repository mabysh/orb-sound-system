use std::sync::mpsc::Sender;
use crate::command::SoundCommand;

#[derive(Clone)]
pub struct OrbSoundSystemHandle {
    pub(crate) tx: Sender<SoundCommand>,
}
