use orb_sound::handle::SoundPriority;
use orb_sound::OrbSoundSystem;
use std::thread;
use std::time::Duration;

fn main() {
    // try initialize sound system with default device
    let mut sound_system_handle = OrbSoundSystem::run().unwrap();
    sound_system_handle
        .play_sound(
            "sounds/test.wav",
            SoundPriority::High,
            None,
        )
        .unwrap();
    thread::sleep(Duration::from_millis(500));
    // pause playback
    sound_system_handle.pause().unwrap();
    thread::sleep(Duration::from_millis(500));
    // resume playback
    sound_system_handle.resume().unwrap();
    thread::sleep(Duration::from_millis(500));
    // adjust volume
    sound_system_handle.adjust_volume(1.0).unwrap();
    thread::sleep(Duration::from_millis(500));
    sound_system_handle.set_volume(0.5).unwrap();
    thread::sleep(Duration::from_secs(3));
}
