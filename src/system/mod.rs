use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

use rodio::{OutputStream, Sink};

use crate::handle::{OrbSoundSystemHandle, PlaySoundCommand, SoundCommand};
use crate::OrbSoundSystemError;
use crate::system::sound::Sound;

mod sound;

/// Type representing Orb's sound system. It runs event loop, receives playback commands, controls
/// playback and decides what file should be played next.
pub struct OrbSoundSystem {
    command_receiver: Receiver<SoundCommand>,
    queue: VecDeque<PlaySoundCommand>,
    current_sound: Option<Sound>,
    sink: Sink,
    _output_stream: OutputStream,
}

impl OrbSoundSystem {
    /// Initialize and run Orb's sound system using default sound device for output. Spawns a thread
    /// and runs event loop on it. Returns either [`OrbSoundSystemHandle`] or some sort of
    /// initialization error.
    pub fn run() -> Result<OrbSoundSystemHandle, OrbSoundSystemError> {
        let (command_sender, command_receiver) = mpsc::channel::<SoundCommand>();
        let (err_sender, err_receiver) = mpsc::channel::<Option<OrbSoundSystemError>>();

        thread::spawn(move || {
            match OrbSoundSystem::init(command_receiver) {
                Ok(system) => {
                    err_sender.send(None).unwrap();
                    system.run_event_loop();
                }
                Err(e) => {
                    err_sender.send(Some(e)).unwrap();
                }
            }
        });

        match err_receiver.recv().unwrap() {
            Some(err) => Err(err),
            None => Ok(OrbSoundSystemHandle { command_sender })
        }
    }

    /// Initialize default sound device.
    fn init(command_receiver: Receiver<SoundCommand>) -> Result<Self, OrbSoundSystemError> {
        // OutputStream must be initialized on event loop thread, otherwise there is no sound output (bug?)
        let (stream, stream_handle) =
            OutputStream::try_default().map_err(|e| OrbSoundSystemError::StreamErr(e))?;
        let sink = Sink::try_new(&stream_handle).map_err(|e| OrbSoundSystemError::PlayErr(e))?;

        Ok(Self {
            command_receiver,
            queue: VecDeque::new(),
            current_sound: None,
            sink,
            _output_stream: stream,
        })
    }

    /// Main event loop. Responsible for:
    ///
    /// - Processing incoming commands
    /// - Filling ring buffer of currently playing sound (if any)
    /// - Playing next sound when previous has finished
    fn run_event_loop(mut self) {
        loop {
            let shutdown = self.process_incoming_commands();
            if shutdown {
                break;
            }

            if let Some(current_sound) = self.current_sound.as_mut() {
                let finished = current_sound.fill_buffer();
                if finished {
                    let _ = self.current_sound.take();
                }
            }

            if let None = self.current_sound {
                if let Some(next_sound) = self.next_sound() {
                    self.current_sound = Some(
                        Sound::play(next_sound.path.as_str(), &self.sink)
                            .expect("Failed to play sound"),
                    );
                }
            }
        }
    }

    /// Process commands coming from channel. Returns true if system should shut down, false
    /// otherwise.
    fn process_incoming_commands(&mut self) -> bool {
        loop {
            match self.command_receiver.try_recv() {
                Ok(command) => match command {
                    SoundCommand::PlaySound(command) => {
                        self.queue.push_back(command);
                    }
                    SoundCommand::SetVolume(value) => {
                        self.sink.set_volume(value);
                    }
                    SoundCommand::AdjustVolume(delta) => {
                        self.sink.set_volume(self.sink.volume() + delta)
                    }
                    SoundCommand::Pause => {
                        self.sink.pause();
                    }
                    SoundCommand::Resume => {
                        self.sink.play();
                    }
                    SoundCommand::Shutdown => {
                        return true;
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

    /// Returns next sound to be played by sorting queue and taking first sound. Checks play
    /// deadlines and drops "expired" sounds
    fn next_sound(&mut self) -> Option<PlaySoundCommand> {
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

#[cfg(test)]
mod test {
    use std::collections::VecDeque;
    use std::sync::mpsc;
    use std::sync::mpsc::Sender;
    use std::time::{Duration, Instant};

    use rodio::{OutputStream, Sink};

    use crate::handle::{PlaySoundCommand, SoundCommand, SoundPriority};
    use crate::system::OrbSoundSystem;

    #[test]
    fn process_commands() {
        let (mut system, command_sender) = mock_system();
        // pause
        command_sender.send(SoundCommand::Pause).unwrap();
        let _ = system.process_incoming_commands();
        assert!(system.sink.is_paused());
        // resume
        command_sender.send(SoundCommand::Resume).unwrap();
        let _ = system.process_incoming_commands();
        assert!(!system.sink.is_paused());
        // set volume
        command_sender.send(SoundCommand::SetVolume(2.0)).unwrap();
        let _ = system.process_incoming_commands();
        assert_eq!(system.sink.volume(), 2.0);
        // adjust volume
        command_sender
            .send(SoundCommand::AdjustVolume(0.5))
            .unwrap();
        let _ = system.process_incoming_commands();
        assert_eq!(system.sink.volume(), 2.5);
        command_sender
            .send(SoundCommand::AdjustVolume(-1.0))
            .unwrap();
        let _ = system.process_incoming_commands();
        assert_eq!(system.sink.volume(), 1.5);
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
            play_deadline: None,
        };

        command_sender.send(SoundCommand::PlaySound(cmd)).unwrap();
        let _ = system.process_incoming_commands();
        assert!(system.next_sound().is_some());
    }

    #[test]
    fn next_sound_after_deadline() {
        let (mut system, _command_sender) = mock_system();
        system.queue.push_back(PlaySoundCommand {
            path: "sounds/test.wav".to_string(),
            priority: SoundPriority::Default,
            play_deadline: Some(Instant::now() - Duration::from_millis(100)),
        });

        assert!(system.next_sound().is_none());
    }

    fn mock_system() -> (OrbSoundSystem, Sender<SoundCommand>) {
        let (tx, rx) = mpsc::channel::<SoundCommand>();
        let system = OrbSoundSystem {
            command_receiver: rx,
            queue: VecDeque::new(),
            sink: Sink::new_idle().0,
            current_sound: None,
            _output_stream: OutputStream::try_default().unwrap().0
        };
        (system, tx)
    }
}
