use std::thread;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

use rodio::{Decoder, OutputStream, Sample, Sink, Source};
use rodio::buffer::SamplesBuffer;
use rodio::source::Buffered;

use crate::command::{PlaySoundCommand, SoundCommand, SoundPriority};
use crate::error::OrbSoundSystemError;
use crate::handle::OrbSoundSystemHandle;

pub struct OrbSoundSystem {
    rx: Receiver<SoundCommand>,
    queue: VecDeque<Sound>,
    sink: Sink,
}

impl OrbSoundSystem {
    pub fn initialize() -> Result<OrbSoundSystemHandle, OrbSoundSystemError> {
        let (_stream, stream_handle) =
            OutputStream::try_default().map_err(|e| OrbSoundSystemError::StreamErr(e))?;
        let sink = Sink::try_new(&stream_handle).map_err(|e| OrbSoundSystemError::PlayErr(e))?;
        let (tx, rx) = mpsc::channel::<SoundCommand>();
        let system = Self {
            rx,
            sink,
            queue: VecDeque::new(),
        };
        thread::spawn(move || loop {
            // drain channel first
        });
        Ok(OrbSoundSystemHandle { tx })
    }

    fn process_incoming_commands(&mut self) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(command) => match command {
                    SoundCommand::PlaySound(command) => match Sound::try_from(command) {
                        Ok(sound) => {
                            self.queue.push_back(sound);
                        }
                        Err(err) => {
                            dbg!(err);
                        }
                    },
                    SoundCommand::SetVolume(_) => {}
                    SoundCommand::AdjustVolume(_) => {}
                    SoundCommand::Pause => {}
                    SoundCommand::Unpause => {}
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
}

struct Sound {
    samples: SamplesBuffer<i16>,
    priority: SoundPriority,
    play_deadline: Option<Instant>,
}

impl TryFrom<PlaySoundCommand> for Sound {
    type Error = OrbSoundSystemError;

    fn try_from(command: PlaySoundCommand) -> Result<Self, Self::Error> {
        let file = File::open(command.path)
            .map_err(|e| OrbSoundSystemError::SoundFileErr(e.to_string()))?;
        let mut source = Decoder::new(BufReader::new(file))
            .map_err(|e| OrbSoundSystemError::SoundFileErr(e.to_string()))?;
        let channels = source.channels();
        let rate = source.sample_rate();
        let samples: Vec<i16> = source.collect();
        Ok(Self {
            samples: SamplesBuffer::new(channels, rate, samples),
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
        (&self.priority, &self.play_deadline).cmp(&(&other.priority, &other.play_deadline))
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
    use rodio::{OutputStream, Sink};
    use rodio::source::Buffered;

    use crate::command::{PlaySoundCommand, SoundPriority};
    use crate::error::OrbSoundSystemError;
    use crate::system::Sound;

    #[test]
    fn test_convert_command() {
        let cmd = PlaySoundCommand {
            path: "sounds/kid_laugh.wav".to_string(),
            priority: SoundPriority::Default,
            max_delay: None,
        };

        let res = Sound::try_from(cmd);
        assert!(res.is_ok());
        let sound = res.unwrap();
        assert_eq!(sound.priority, SoundPriority::Default);
        assert_eq!(sound.play_deadline, None);
    }

    #[test]
    fn play_sound() {
        let cmd = PlaySoundCommand {
            path: "sounds/kid_laugh.wav".to_string(),
            priority: SoundPriority::Default,
            max_delay: None,
        };

        // FIXME use system to play it
        let sound = Sound::try_from(cmd).unwrap();
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();
        sink.append(sound.samples);
        sink.sleep_until_end();
    }
}
