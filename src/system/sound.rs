use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

use rodio::{Decoder, Sample, Sink, Source};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::OrbSoundSystemError;

// Buffer that may contain up to 50ms of wav data with 44100 sample rate
const BUFFER_CAPACITY: usize = 44100 / 20 * 2;

/// Type representing sound currently being played. Backed by ring buffer and consists of two parts:
///
/// - A consumer part represented by [`SoundConsumer`] which is used to read sound samples.
/// - A producer part represented by [`SoundProducer`] which is used to write sound samples.
pub(crate) type Sound = SoundProducer<Decoder<BufReader<File>>>;

/// Producer part of a ring buffer. User of the type is responsible for keeping ring buffer full
/// using [`SoundProducer::fill_buffer()`] associated function.
pub(crate) struct SoundProducer<I> {
    /// Source of sound samples
    reader: I,
    /// Ring buffer producer
    buffer: Producer<i16>,
}

impl SoundProducer<Decoder<BufReader<File>>> {
    /// Start playing a file located by `path`. Creates producer and consumer parts of ring buffer
    /// and fills it with data. Consumer pushed to the output stream and producer returned to the
    /// caller which is responsible for keeping ring buffer full.
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
        let source = SoundConsumer {
            buffer: consumer,
            channels: decoder.channels(),
            sample_rate: decoder.sample_rate(),
        };

        let mut sound = SoundProducer {
            reader: decoder,
            buffer: producer,
        };
        sound.fill_buffer();
        sink.append(source);

        Ok(sound)
    }
}

impl<I> SoundProducer<I>
where
    I: Iterator<Item = i16>,
{
    /// Fill available slots of ring buffer with sound samples from underlying reader. Returns true
    /// if underlying reader is out of data.
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

/// Consumer part of ring buffer.
struct SoundConsumer {
    buffer: Consumer<i16>,
    channels: u16,
    sample_rate: u32,
}

impl Iterator for SoundConsumer {
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

impl Source for SoundConsumer {
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
    use crate::system::sound::{SoundConsumer, SoundProducer};

    #[test]
    fn source_iterator() {
        let (mut producer, consumer) = RingBuffer::new(2);
        let mut source = SoundConsumer {
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
        let mut source = SoundConsumer {
            buffer: consumer,
            channels: 0,
            sample_rate: 0
        };
        let mut sound = SoundProducer {
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
        let mut sound = SoundProducer::play("sounds/test.wav", &sink).unwrap();
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