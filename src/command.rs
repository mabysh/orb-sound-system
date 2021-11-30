use std::time::Duration;

pub enum SoundCommand {
    PlaySound(PlaySoundCommand),
    SetVolume(f32),
    AdjustVolume(f32),
    Pause,
    Unpause,
}

pub struct PlaySoundCommand {
    pub path: String,
    pub priority: SoundPriority,
    pub max_delay: Option<Duration>,
}

// note about ordering top-to-bottom by discriminant
#[derive(PartialOrd, PartialEq)]
pub enum SoundPriority {
    Urgent,
    High,
    Default,
}
