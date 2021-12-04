use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

use rodio::{Decoder, OutputStream, Sink};

use crate::handle::{OrbSoundSystemHandle, PlaySoundCommand, SoundCommand, SoundPriority};
use crate::OrbSoundSystemError;

pub struct OrbSoundSystem {
    command_receiver: Receiver<SoundCommand>,
    queue: VecDeque<Sound>,
    soundtrack: Sink,
}

impl OrbSoundSystem {
    pub fn initialize() -> Result<OrbSoundSystemHandle, OrbSoundSystemError> {
        let (_stream, stream_handle) =
            OutputStream::try_default().map_err(|e| OrbSoundSystemError::StreamErr(e))?;
        let sink = Sink::try_new(&stream_handle).map_err(|e| OrbSoundSystemError::PlayErr(e))?;
        let (command_sender, command_receiver) = mpsc::channel::<SoundCommand>();

        let system = Self {
            command_receiver,
            soundtrack: sink,
            queue: VecDeque::new(),
        };
        system.run();

        Ok(OrbSoundSystemHandle { command_sender })
    }

    fn run(mut self) {
        thread::spawn(move || loop {
            let shutdown = self.process_incoming_commands();
            if shutdown {
                break;
            }

            if self.soundtrack.empty() {
                if let Some(next_sound) = self.next_sound() {
                    self.soundtrack.append(next_sound.source);
                }
            }
        });
    }

    fn process_incoming_commands(&mut self) -> bool {
        loop {
            match self.command_receiver.try_recv() {
                Ok(command) => match command {
                    SoundCommand::PlaySound(command) => {
                        let sound = Sound::try_from(command).expect("Failed to construct sound");
                        self.queue.push_back(sound);
                    }
                    SoundCommand::SetVolume(value) => {
                        self.soundtrack.set_volume(value);
                    }
                    SoundCommand::AdjustVolume(delta) => {
                        self.soundtrack.set_volume(self.soundtrack.volume() + delta);
                    }
                    SoundCommand::Pause => {
                        self.soundtrack.pause();
                    }
                    SoundCommand::Unpause => {
                        self.soundtrack.play();
                    }
                },
                Err(err) => {
                    return match err {
                        TryRecvError::Disconnected => true,
                        _ => false,
                    }
                }
            };
        }
    }

    fn next_sound(&mut self) -> Option<Sound> {
        self.queue.make_contiguous().sort();
        while let Some(next) = self.queue.pop_front() {
            match next.play_deadline {
                Some(deadline) => {
                    if Instant::now() <= deadline {
                        return Some(next);
                    }
                }
                None => return Some(next),
            }
        }
        None
    }
}

struct Sound {
    source: Decoder<BufReader<File>>,
    priority: SoundPriority,
    play_deadline: Option<Instant>,
}

impl TryFrom<PlaySoundCommand> for Sound {
    type Error = OrbSoundSystemError;

    fn try_from(command: PlaySoundCommand) -> Result<Self, Self::Error> {
        // Buffer that may contain up to 50ms of wav data with 44100 sample rate
        const BUFFER_CAPACITY: usize = 44100 / 20 * 4;

        let file = File::open(&command.path).map_err(|e| {
            OrbSoundSystemError::SoundFileErr(format!("{}: {}", &command.path, e.to_string()))
        })?;
        let source =
            Decoder::new_wav(BufReader::with_capacity(BUFFER_CAPACITY, file)).map_err(|e| {
                OrbSoundSystemError::SoundFileErr(format!("{}: {}", &command.path, e.to_string()))
            })?;
        Ok(Self {
            source,
            priority: command.priority,
            play_deadline: command.max_delay.map(|delay| Instant::now() + delay),
        })
    }
}

impl PartialOrd for Sound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Sound {
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

impl PartialEq<Self> for Sound {
    fn eq(&self, other: &Self) -> bool {
        self.priority.eq(&other.priority) && self.play_deadline.eq(&other.play_deadline)
    }
}

impl Eq for Sound {}

#[cfg(test)]
mod test {
    use std::collections::VecDeque;
    use std::sync::mpsc;
    use std::sync::mpsc::Sender;
    use std::time::{Duration, Instant};

    use rodio::Sink;

    use crate::handle::{PlaySoundCommand, SoundCommand, SoundPriority};
    use crate::system::{OrbSoundSystem, Sound};

    #[test]
    fn convert_command() {
        let cmd = PlaySoundCommand {
            path: "sounds/test.wav".to_string(),
            priority: SoundPriority::High,
            max_delay: Some(Duration::from_secs(2)),
        };

        let res = Sound::try_from(cmd);
        assert!(res.is_ok());
        let sound = res.unwrap();
        assert_eq!(sound.priority, SoundPriority::High);
        assert!(Some(Instant::now() + Duration::from_secs(2)) > sound.play_deadline);
        assert!(!sound.source.collect::<Vec<i16>>().is_empty());
    }

    #[test]
    fn sound_priority_sorting() {
        let mut queue = VecDeque::new();
        queue.push_back(mock_sound(SoundPriority::Default, None));
        queue.push_back(mock_sound(
            SoundPriority::Default,
            Some(Instant::now() + Duration::from_secs(2)),
        ));
        queue.push_back(mock_sound(SoundPriority::High, None));
        queue.push_back(mock_sound(
            SoundPriority::High,
            Some(Instant::now() + Duration::from_secs(5)),
        ));
        queue.push_back(mock_sound(
            SoundPriority::High,
            Some(Instant::now() + Duration::from_secs(3)),
        ));
        queue.push_back(mock_sound(SoundPriority::Urgent, None));
        queue.make_contiguous().sort();

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

    #[test]
    fn process_commands() {
        let (mut system, command_sender) = mock_system();
        // pause
        command_sender.send(SoundCommand::Pause).unwrap();
        let _ = system.process_incoming_commands();
        assert!(system.soundtrack.is_paused());
        // resume
        command_sender.send(SoundCommand::Unpause).unwrap();
        let _ = system.process_incoming_commands();
        assert!(!system.soundtrack.is_paused());
        // set volume
        command_sender.send(SoundCommand::SetVolume(2.0)).unwrap();
        let _ = system.process_incoming_commands();
        assert_eq!(system.soundtrack.volume(), 2.0);
        // adjust volume
        command_sender
            .send(SoundCommand::AdjustVolume(0.5))
            .unwrap();
        let _ = system.process_incoming_commands();
        assert_eq!(system.soundtrack.volume(), 2.5);
        command_sender
            .send(SoundCommand::AdjustVolume(-1.0))
            .unwrap();
        let _ = system.process_incoming_commands();
        assert_eq!(system.soundtrack.volume(), 1.5);
    }

    #[test]
    fn shutdown() {
        let (mut system, command_sender) = mock_system();
        std::mem::drop(command_sender);
        let shutdown = system.process_incoming_commands();
        assert!(shutdown);
    }

    #[test]
    fn next_sound() {
        let (mut system, command_sender) = mock_system();
        let cmd = PlaySoundCommand {
            path: "sounds/test.wav".to_string(),
            priority: SoundPriority::Default,
            max_delay: None,
        };

        command_sender.send(SoundCommand::PlaySound(cmd)).unwrap();
        let _ = system.process_incoming_commands();
        assert!(system.next_sound().is_some());
    }

    #[test]
    fn next_sound_after_deadline() {
        let (mut system, _command_sender) = mock_system();
        system.queue.push_back(mock_sound(
            SoundPriority::Default,
            Some(Instant::now() - Duration::from_millis(100)),
        ));

        assert!(system.next_sound().is_none());
    }

    fn mock_system() -> (OrbSoundSystem, Sender<SoundCommand>) {
        let (tx, rx) = mpsc::channel::<SoundCommand>();
        let system = OrbSoundSystem {
            command_receiver: rx,
            soundtrack: Sink::new_idle().0,
            queue: VecDeque::new(),
        };
        (system, tx)
    }

    fn mock_sound(priority: SoundPriority, play_deadline: Option<Instant>) -> Sound {
        let cmd = PlaySoundCommand {
            path: "sounds/test.wav".to_string(),
            priority: SoundPriority::Default,
            max_delay: None,
        };
        let mut sound = Sound::try_from(cmd).unwrap();
        sound.priority = priority;
        sound.play_deadline = play_deadline;
        sound
    }
}
