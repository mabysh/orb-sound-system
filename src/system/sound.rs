use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use rodio::{Decoder, Sample, Sink, Source};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::OrbSoundSystemError;

// Buffer that may contain up to 50ms of wav data with 44100 sample rate
const BUFFER_CAPACITY: usize = 44100 / 20 * 2;

pub(crate) type Sound = OrbSound<Decoder<BufReader<File>>>;

pub(crate) struct OrbSound<I> {
    reader: I,
    buffer: Producer<i16>,
}

impl OrbSound<Decoder<BufReader<File>>> {
    pub fn play(
        path: &str,
        sink: &Sink,
    ) -> Result<Sound, OrbSoundSystemError> {
        let (producer, consumer) = RingBuffer::new(BUFFER_CAPACITY);

        let file = File::open(path).map_err(|e| {
            OrbSoundSystemError::SoundFileErr(format!("{}: {}", path, e.to_string()))
        })?;
        let decoder = Decoder::new_wav(BufReader::new(file)).map_err(|e| {
            OrbSoundSystemError::SoundFileErr(format!("{}: {}", path, e.to_string()))
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

#[cfg(test)]
mod test {
    use std::time::Duration;
    use rodio::{OutputStream, Sample, Sink};
    use rodio::buffer::SamplesBuffer;
    use rtrb::RingBuffer;

    use crate::OrbSoundSystemError;
    use crate::system::sound::{OrbSound, OrbSoundSource};

    #[test]
    fn source_iterator() {
        let (mut producer, consumer) = RingBuffer::new(2);
        let mut source = OrbSoundSource {
            buffer: consumer,
            channels: 0,
            sample_rate: 0
        };
        producer.push(1).unwrap();
        producer.push(2).unwrap();
        assert_eq!(source.next(), Some(1));
        assert_eq!(source.next(), Some(2));
        // buffer underrun
        assert_eq!(source.next(), Some(<i16 as Sample>::zero_value()));
        drop(producer);
        assert_eq!(source.next(), None);
    }

    #[test]
    fn fill_buffer() {
        let reader = SamplesBuffer::new(2, 1, vec![1i16; 15]);
        let (producer, consumer) = RingBuffer::new(10);
        let mut source = OrbSoundSource {
            buffer: consumer,
            channels: 0,
            sample_rate: 0
        };
        let mut sound = OrbSound {
            reader,
            buffer: producer
        };
        let out_of_data = sound.fill_buffer();
        assert!(!out_of_data);
        assert_eq!(source.buffer.slots(), 10);
        for _ in 0..10 {
            assert_eq!(source.next(), Some(1));
        }
        assert_eq!(source.next(), Some(<i16 as Sample>::zero_value()));
        let out_of_data = sound.fill_buffer();
        assert!(out_of_data);
        assert_eq!(source.buffer.slots(), 5);
        drop(sound);
        for _ in 0..5 {
            assert_eq!(source.next(), Some(1));
        }
        assert_eq!(source.next(), None);
    }

    #[test]
    #[ignore]
    fn ring_buffer() {
        let (_stream, stream_handle) =
            OutputStream::try_default().map_err(|e| OrbSoundSystemError::StreamErr(e)).unwrap();
        let sink = Sink::try_new(&stream_handle).map_err(|e| OrbSoundSystemError::PlayErr(e)).unwrap();
        let mut sound = OrbSound::play("sounds/test.wav", &sink).unwrap();
        loop {
            let finished = sound.fill_buffer();
            if finished  {
                break;
            }
            // set sleep duration to 50ms to hear buffer underrun glitches
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}