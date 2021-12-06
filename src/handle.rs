use std::cmp::Ordering;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::OrbSoundSystemError;

/// Handle to the sound system. All communication with sound system is done using methods of this
/// type. Can be safely cloned and moved between threads.
#[derive(Clone)]
pub struct OrbSoundSystemHandle {
    pub(crate) command_sender: Sender<SoundCommand>,
}

impl OrbSoundSystemHandle {
    /// Request playback of the file located by given `path`. Usually it plays immediately but
    /// if there are multiple threads using this method concurrently, final order of played files
    /// will be determined based on `priority` and `max_delay` parameters. Files are never played
    /// simultaneously.
    ///
    /// It is guaranteed that file will not be played after deadline specified by `max_delay` duration.
    pub fn play_sound(
        &mut self,
        path: &str,
        priority: SoundPriority,
        max_delay: Option<Duration>,
    ) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::PlaySound(PlaySoundCommand {
            path: path.to_string(),
            priority,
            play_deadline: max_delay.map(|delay| Instant::now() + delay),
        }))
    }

    /// Set particular volume. If not changed, volume of a sound is equal to 1.0. Thus setting
    /// volume to 2.0 will make it twice lauder.
    pub fn set_volume(&mut self, value: f32) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::SetVolume(value))
    }

    /// Adjust volume by given amount. Positive to make it lauder, negative to make it quieter.
    pub fn adjust_volume(&mut self, delta: f32) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::AdjustVolume(delta))
    }

    /// Pause playback. Does nothing if already paused.
    pub fn pause(&mut self) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::Pause)
    }

    /// Resume playback. Does nothing if not paused.
    pub fn resume(&mut self) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::Resume)
    }

    /// Shutdown the system by stopping its event loop. Using handle after this call will return
    /// error.
    pub fn shutdown(&mut self) -> Result<(), OrbSoundSystemError> {
        self.send_command(SoundCommand::Shutdown)
    }

    fn send_command(&mut self, command: SoundCommand) -> Result<(), OrbSoundSystemError> {
        self.command_sender
            .send(command)
            .map_err(|_| OrbSoundSystemError::SystemIsDown)
    }
}

/// Sound command.
#[derive(Debug, PartialEq)]
pub(crate) enum SoundCommand {
    PlaySound(PlaySoundCommand),
    SetVolume(f32),
    AdjustVolume(f32),
    Pause,
    Resume,
    Shutdown,
}

/// Associated struct for [`SoundCommand::PlaySound`] command.
#[derive(Debug)]
pub(crate) struct PlaySoundCommand {
    /// Path to file in filesystem
    pub path: String,
    /// Sound priority
    pub priority: SoundPriority,
    /// Deadline after which sound will not be played
    pub play_deadline: Option<Instant>,
}

/// Sound priority. Used to determine what sound should be played next.
#[derive(PartialOrd, PartialEq, Ord, Eq, Debug, Clone)]
pub enum SoundPriority {
    // Note! Do not change positions of enums as it will affect
    // sorting of files being played
    Urgent,
    High,
    Default,
}

impl PartialOrd for PlaySoundCommand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// [`PlaySoundCommand`]s are sorted by:
///
/// - priorities
/// - if priorities are not equal, by play deadline. Files which have smaller time window to play
///   will have higher priority
impl Ord for PlaySoundCommand {
    fn cmp(&self, other: &Self) -> Ordering {
        let by_priority = self.priority.cmp(&other.priority);
        let by_deadline = if self.play_deadline.is_some() && other.play_deadline.is_some() {
            self.play_deadline.cmp(&other.play_deadline)
        } else if self.play_deadline.is_some() {
            Ordering::Less
        } else {
            Ordering::Greater
        };
        by_priority.then(by_deadline)
    }
}

impl PartialEq<Self> for PlaySoundCommand {
    fn eq(&self, other: &Self) -> bool {
        self.priority.eq(&other.priority) && self.play_deadline.eq(&other.play_deadline)
    }
}

impl Eq for PlaySoundCommand {}

#[cfg(test)]
mod test {
    use std::time::{Duration, Instant};

    use crate::handle::{OrbSoundSystemHandle, PlaySoundCommand, SoundCommand, SoundPriority};

    #[test]
    fn test_handle() {
        let (tx, rx) = std::sync::mpsc::channel::<SoundCommand>();
        let mut handle = OrbSoundSystemHandle { command_sender: tx };
        handle
            .play_sound(
                String::new().as_str(),
                SoundPriority::High,
                Some(Duration::from_secs(1)),
            )
            .unwrap();
        if let SoundCommand::PlaySound(command) = rx.recv().unwrap() {
            assert_eq!(
                command,
                PlaySoundCommand {
                    path: String::new(),
                    priority: SoundPriority::High,
                    play_deadline: command.play_deadline.clone(),
                }
            );
        } else {
            panic!()
        }

        handle.set_volume(2.0).unwrap();
        assert_eq!(rx.recv().unwrap(), SoundCommand::SetVolume(2.0));
        handle.adjust_volume(-0.5).unwrap();
        assert_eq!(rx.recv().unwrap(), SoundCommand::AdjustVolume(-0.5));
        handle.pause().unwrap();
        assert_eq!(rx.recv().unwrap(), SoundCommand::Pause);
        handle.resume().unwrap();
        assert_eq!(rx.recv().unwrap(), SoundCommand::Resume);
    }

    #[test]
    fn sound_priority_sorting() {
        let mut queue = Vec::new();
        queue.push(PlaySoundCommand {
            path: String::new(),
            priority: SoundPriority::Default,
            play_deadline: None,
        });
        queue.push(PlaySoundCommand {
            path: String::new(),
            priority: SoundPriority::Default,
            play_deadline: Some(Instant::now() + Duration::from_secs(2)),
        });
        queue.push(PlaySoundCommand {
            path: String::new(),
            priority: SoundPriority::High,
            play_deadline: None,
        });
        queue.push(PlaySoundCommand {
            path: String::new(),
            priority: SoundPriority::High,
            play_deadline: Some(Instant::now() + Duration::from_secs(5)),
        });
        queue.push(PlaySoundCommand {
            path: String::new(),
            priority: SoundPriority::High,
            play_deadline: Some(Instant::now() + Duration::from_secs(3)),
        });
        queue.push(PlaySoundCommand {
            path: String::new(),
            priority: SoundPriority::Urgent,
            play_deadline: None,
        });
        queue.sort();

        assert_eq!(queue.get(0).unwrap().priority, SoundPriority::Urgent);
        assert_eq!(queue.get(0).unwrap().play_deadline, None);
        assert_eq!(queue.get(1).unwrap().priority, SoundPriority::High);
        assert!(
            Some(Instant::now() + Duration::from_secs(3)) > queue.get(1).unwrap().play_deadline
        );
        assert_eq!(queue.get(2).unwrap().priority, SoundPriority::High);
        assert!(
            Some(Instant::now() + Duration::from_secs(5)) > queue.get(2).unwrap().play_deadline
        );
        assert_eq!(queue.get(3).unwrap().priority, SoundPriority::High);
        assert_eq!(queue.get(3).unwrap().play_deadline, None);
        assert_eq!(queue.get(4).unwrap().priority, SoundPriority::Default);
        assert!(
            Some(Instant::now() + Duration::from_secs(2)) > queue.get(4).unwrap().play_deadline
        );
        assert_eq!(queue.get(5).unwrap().priority, SoundPriority::Default);
        assert_eq!(queue.get(5).unwrap().play_deadline, None);
    }
}
