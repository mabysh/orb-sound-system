use std::sync::mpsc::Sender;
use std::time::Duration;

#[derive(Clone)]
pub struct OrbSoundSystemHandle {
    pub(crate) command_sender: Sender<SoundCommand>,
}

impl OrbSoundSystemHandle {
    pub fn play_sound(&mut self, path: &str, priority: SoundPriority, max_delay: Option<Duration>) {
        self.send_command(SoundCommand::PlaySound(PlaySoundCommand {
            path: path.to_string(),
            priority,
            max_delay
        }));
    }

    pub fn set_volume(&mut self, value: f32) {
        self.send_command(SoundCommand::SetVolume(value));
    }

    pub fn adjust_volume(&mut self, delta: f32) {
        self.send_command(SoundCommand::AdjustVolume(delta));
    }

    pub fn pause(&mut self) {
        self.send_command(SoundCommand::Pause);
    }

    pub fn unpause(&mut self) {
        self.send_command(SoundCommand::Unpause);
    }

    fn send_command(&mut self, command: SoundCommand) {
        if let Err(err) = self.command_sender.send(command) {
            dbg!(err);
        }
    }
}

pub(crate) enum SoundCommand {
    PlaySound(PlaySoundCommand),
    SetVolume(f32),
    AdjustVolume(f32),
    Pause,
    Unpause,
}

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

