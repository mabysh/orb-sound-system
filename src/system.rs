use std::collections::VecDeque;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

use rodio::source::Buffered;
use rodio::{Decoder, OutputStream, Sample, Sink, Source};

use crate::command::{PlaySoundCommand, SoundCommand, SoundPriority};
use crate::error::OrbSoundSystemError;
use crate::handle::OrbSoundSystemHandle;

pub struct OrbSoundSystem {
    rx: Receiver<SoundCommand>,
    sounds: VecDeque<PlaySoundCommand>,
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
            sounds: VecDeque::new(),
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
                    SoundCommand::PlaySound(command) => {

                    }
                    SoundCommand::SetVolume(_) => {}
                    SoundCommand::AdjustVolume(_) => {}
                    SoundCommand::Pause => {}
                    SoundCommand::Unpause => {}
                },
                Err(err) => return match err {
                    TryRecvError::Disconnected => true,
                    _ => false,
                },
            };
        }
    }
}

struct Sound {
    source: Buffered<Decoder<BufReader<File>>>,
    priority: SoundPriority,
    play_deadline: Option<Instant>,
}

impl TryFrom<PlaySoundCommand> for Sound {
    type Error = OrbSoundSystemError;

    fn try_from(command: PlaySoundCommand) -> Result<Self, Self::Error> {
        let file = File::open(command.path)
            .map_err(|e| OrbSoundSystemError::SoundFileErr(e.to_string()))?;
        let reader = BufReader::new(file);
        let source =
            Decoder::new(reader).map_err(|e| OrbSoundSystemError::SoundFileErr(e.to_string()))?;
        Ok(Self {
            source: source.buffered(),
            priority: command.priority,
            play_deadline: command.max_delay.map(|delay| Instant::now() + delay),
        })
    }
}
