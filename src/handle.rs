use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::OrbSoundSystemError;

#[derive(Clone)]
pub struct OrbSoundSystemHandle {
    pub(crate) command_sender: Sender<SoundCommand>,
}

impl OrbSoundSystemHandle {
    pub fn play_sound(
        &mut self,
        path: &str,
        priority: SoundPriority,
        max_delay: Option<Duration>,
    ) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::PlaySound(PlaySoundCommand {
            path: path.to_string(),
            priority,
            max_delay,
        }))
    }

    pub fn set_volume(&mut self, value: f32) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::SetVolume(value))
    }

    pub fn adjust_volume(&mut self, delta: f32) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::AdjustVolume(delta))
    }

    pub fn pause(&mut self) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::Pause)
    }

    pub fn unpause(&mut self) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::Unpause)
    }

    fn send_command(&mut self, command: SoundCommand) -> Result<(), OrbSoundSystemError> {
        self.command_sender
            .send(command)
            .map_err(|_| OrbSoundSystemError::SystemIsDown)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum SoundCommand {
    PlaySound(PlaySoundCommand),
    SetVolume(f32),
    AdjustVolume(f32),
    Pause,
    Unpause,
}

#[derive(Debug, PartialEq)]
pub(crate) struct PlaySoundCommand {
    pub path: String,
    pub priority: SoundPriority,
    pub max_delay: Option<Duration>,
}

// note about ordering top-to-bottom by discriminant
#[derive(PartialOrd, PartialEq, Ord, Eq, Debug)]
pub enum SoundPriority {
    Urgent,
    High,
    Default,
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::handle::{OrbSoundSystemHandle, PlaySoundCommand, SoundCommand, SoundPriority};

    #[test]
    fn test_handle() {
        let (tx, rx) = std::sync::mpsc::channel::<SoundCommand>();
        let mut handle = OrbSoundSystemHandle { command_sender: tx };
        handle
            .play_sound("abc", SoundPriority::High, Some(Duration::from_secs(1)))
            .unwrap();
        assert_eq!(
            rx.recv().unwrap(),
            SoundCommand::PlaySound(PlaySoundCommand {
                path: "abc".to_string(),
                priority: SoundPriority::High,
                max_delay: Some(Duration::from_secs(1))
            })
        );
        handle.set_volume(2.0).unwrap();
        assert_eq!(rx.recv().unwrap(), SoundCommand::SetVolume(2.0));
        handle.adjust_volume(-0.5).unwrap();
        assert_eq!(rx.recv().unwrap(), SoundCommand::AdjustVolume(-0.5));
        handle.pause().unwrap();
        assert_eq!(rx.recv().unwrap(), SoundCommand::Pause);
        handle.unpause().unwrap();
        assert_eq!(rx.recv().unwrap(), SoundCommand::Unpause);
    }
}
