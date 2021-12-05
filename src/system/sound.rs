use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use rodio::{Decoder, Sample, Sink, Source};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::handle::{PlaySoundCommand};
use crate::OrbSoundSystemError;

pub(crate) struct OrbSound<I> {
    reader: I,
    buffer: Producer<i16>,
}

impl OrbSound<Decoder<BufReader<File>>> {
    pub fn play(
        cmd: PlaySoundCommand,
        sink: &Sink,
    ) -> Result<OrbSound<Decoder<BufReader<File>>>, OrbSoundSystemError> {
        // Buffer that may contain up to 50ms of wav data with 44100 sample rate
        const BUFFER_CAPACITY: usize = 44100 / 20 * 2;
        let (producer, consumer) = RingBuffer::new(BUFFER_CAPACITY);

        let file = File::open(&cmd.path).map_err(|e| {
            OrbSoundSystemError::SoundFileErr(format!("{}: {}", &cmd.path, e.to_string()))
        })?;
        let decoder = Decoder::new_wav(BufReader::new(file)).map_err(|e| {
            OrbSoundSystemError::SoundFileErr(format!("{}: {}", &cmd.path, e.to_string()))
        })?;
        let source = OrbSoundSource {
            buffer: consumer,
            channels: decoder.channels(),
            sample_rate: decoder.sample_rate(),
        };

        let mut sound = OrbSound {
            reader: decoder,
            buffer: producer,
        };
        sound.fill_buffer();
        sink.append(source);

        Ok(sound)
    }
}

impl<I> OrbSound<I>
where
    I: Iterator<Item = i16>,
{
    pub fn fill_buffer(&mut self) -> bool {
        let slots_available = self.buffer.slots();
        for _ in 0..slots_available {
            if let Some(sample) = self.reader.next() {
                // Unwrap is safe here because we checked slots availability
                self.buffer.push(sample).unwrap();
            } else {
                return true;
            }
        }
        false
    }
}

struct OrbSoundSource {
    buffer: Consumer<i16>,
    channels: u16,
    sample_rate: u32,
}

impl Iterator for OrbSoundSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(sample) = self.buffer.pop() {
            return Some(sample);
        }
        // Producer was dropped. Usually it means end of file
        if self.buffer.is_abandoned() {
            return None;
        }
        // Reaching here means buffer underrun condition. Producing silence
        Some(<i16 as Sample>::zero_value())
    }
}

impl Source for OrbSoundSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
