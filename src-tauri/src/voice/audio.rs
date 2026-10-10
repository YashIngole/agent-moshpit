//! Bounded conversion on the device callback. No I/O or inference here.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub const RATE: usize = 16_000;
pub const MAX_SECONDS: usize = 60;
pub const MAX_SAMPLES: usize = RATE * MAX_SECONDS;

pub struct Capture {
    pub pcm: Vec<f32>,
    channels: usize,
    rate: u32,
    frame_sum: f32,
    channel: usize,
    frame: u64,
    previous: f32,
    next: f64,
    pub full: bool,
}

impl Capture {
    pub fn new(channels: u16, rate: u32) -> Result<Self, String> {
        if channels == 0 || channels > 32 || !(8_000..=384_000).contains(&rate) {
            return Err(
                "This microphone's format is unsupported. Try another default input device.".into(),
            );
        }
        Ok(Self {
            pcm: Vec::with_capacity(MAX_SAMPLES),
            channels: usize::from(channels),
            rate,
            frame_sum: 0.0,
            channel: 0,
            frame: 0,
            previous: 0.0,
            next: 0.0,
            full: false,
        })
    }

    pub fn push<T: Sample>(&mut self, samples: &[T])
    where
        f32: FromSample<T>,
    {
        for &sample in samples {
            if self.full {
                break;
            }
            let value = f32::from_sample(sample);
            self.frame_sum += if value.is_finite() {
                value.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            self.channel += 1;
            if self.channel != self.channels {
                continue;
            }
            let mono = self.frame_sum / self.channels as f32;
            self.frame_sum = 0.0;
            self.channel = 0;
            // Linear interpolation at the actual device rate, across callback boundaries.
            while self.next <= self.frame as f64 && self.pcm.len() < MAX_SAMPLES {
                let fraction =
                    (self.next - self.frame.saturating_sub(1) as f64).clamp(0.0, 1.0) as f32;
                self.pcm
                    .push(self.previous + (mono - self.previous) * fraction);
                self.next += f64::from(self.rate) / RATE as f64;
            }
            self.previous = mono;
            self.frame += 1;
            self.full = self.pcm.len() == MAX_SAMPLES;
        }
    }
}

pub fn speech(pcm: &[f32]) -> Result<(), String> {
    if pcm.len() < RATE / 2 {
        return Err(
            "That recording was too short. Speak for at least half a second, then stop.".into(),
        );
    }
    if pcm.len() > MAX_SAMPLES || pcm.iter().any(|v| !v.is_finite()) {
        return Err("The microphone returned invalid audio. Try another input device.".into());
    }
    // Require at least 200ms of 20ms windows that stand out, not just a single click.
    let levels: Vec<f64> = pcm
        .chunks(RATE / 50)
        .map(|chunk| (chunk.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / chunk.len() as f64).sqrt())
        .collect();
    // A quiet microphone is fine, but steady room noise is not speech: words must stand
    // clearly above this recording's own quietest moments (its pauses), as well as above
    // a fixed floor. Otherwise amplified hiss reaches the model, which invents words in it.
    let mut sorted = levels.clone();
    sorted.sort_by(f64::total_cmp);
    let background = sorted[sorted.len() / 10];
    let threshold = (background * 3.0).max(0.002);
    let audible = levels.iter().filter(|level| **level > threshold).count();
    if audible < 10 {
        return Err(
            "No clear speech was heard. Choose your microphone in Voice input and check that the input meter moves when you speak.".into(),
        );
    }
    Ok(())
}

pub struct DeviceCapture {
    pub stream: cpal::Stream,
    pub audio: Arc<Mutex<Capture>>,
    pub failed: Arc<AtomicBool>,
    pub count: Arc<AtomicUsize>,
    pub level: Arc<AtomicU32>,
    pub name: String,
}

/// Give quiet speech usable amplitude after the silence check, with bounded gain.
pub fn normalize(pcm: &mut [f32]) {
    let peak = pcm.iter().fold(0.0f32, |peak, v| peak.max(v.abs()));
    if peak > 0.0 && peak < 0.5 {
        let gain = (0.5 / peak).min(20.0);
        for sample in pcm {
            *sample *= gain;
        }
    }
}

#[derive(Default, serde::Serialize)]
pub struct Inputs {
    pub devices: Vec<String>,
    pub default: Option<String>,
}

/// Enumeration does not open a recording stream or request microphone capture.
pub fn inputs() -> Result<Inputs, String> {
    let host = cpal::default_host();
    let default = host.default_input_device().and_then(|d| d.name().ok());
    let mut devices = Vec::new();
    for device in host.input_devices().map_err(|e| {
        format!("Microphones could not be listed: {e}. Reconnect your microphone and refresh.")
    })? {
        if let Ok(name) = device.name() {
            if !devices.contains(&name) {
                devices.push(name);
            }
        }
    }
    Ok(Inputs { devices, default })
}

pub fn open(selected: Option<&str>) -> Result<DeviceCapture, String> {
    let host = cpal::default_host();
    let device = match selected.filter(|name| !name.is_empty()) {
        Some(name) => host.input_devices().map_err(|e| format!("Microphones could not be listed: {e}"))?
            .find(|d| d.name().is_ok_and(|n| n == name))
            .ok_or("The selected microphone is disconnected. Open Voice input, refresh microphones and choose an available input.")?,
        None => host.default_input_device().ok_or(
            "No microphone was found. Connect one, then choose it in Voice input.",
        )?,
    };
    let name = device
        .name()
        .unwrap_or_else(|_| "Selected microphone".into());
    let config = device.default_input_config().map_err(|e| {
        format!("The microphone {name} is unavailable: {e}. Check system microphone access.")
    })?;
    let capture = Arc::new(Mutex::new(Capture::new(
        config.channels(),
        config.sample_rate().0,
    )?));
    let failed = Arc::new(AtomicBool::new(false));
    let count = Arc::new(AtomicUsize::new(0));
    let level = Arc::new(AtomicU32::new(0));
    let stream_config = config.clone().into();
    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => build::<i8>(&device, &stream_config, &capture, &failed, &count, &level),
        cpal::SampleFormat::I16 => build::<i16>(&device, &stream_config, &capture, &failed, &count, &level),
        cpal::SampleFormat::I32 => build::<i32>(&device, &stream_config, &capture, &failed, &count, &level),
        cpal::SampleFormat::I64 => build::<i64>(&device, &stream_config, &capture, &failed, &count, &level),
        cpal::SampleFormat::U8 => build::<u8>(&device, &stream_config, &capture, &failed, &count, &level),
        cpal::SampleFormat::U16 => build::<u16>(&device, &stream_config, &capture, &failed, &count, &level),
        cpal::SampleFormat::U32 => build::<u32>(&device, &stream_config, &capture, &failed, &count, &level),
        cpal::SampleFormat::U64 => build::<u64>(&device, &stream_config, &capture, &failed, &count, &level),
        cpal::SampleFormat::F32 => build::<f32>(&device, &stream_config, &capture, &failed, &count, &level),
        cpal::SampleFormat::F64 => build::<f64>(&device, &stream_config, &capture, &failed, &count, &level),
        _ => return Err("This microphone's sample format is unsupported. Choose another input device in Voice input.".into()),
    }.map_err(|e| format!("The microphone {name} could not start: {e}. Check system microphone access or choose another input in Voice input."))?;
    stream.play().map_err(|e| {
        format!("The microphone could not record: {e}. Check system microphone access.")
    })?;
    Ok(DeviceCapture {
        stream,
        audio: capture,
        failed,
        count,
        level,
        name,
    })
}

fn build<T: SizedSample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    capture: &Arc<Mutex<Capture>>,
    failed: &Arc<AtomicBool>,
    count: &Arc<AtomicUsize>,
    level: &Arc<AtomicU32>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    f32: FromSample<T>,
{
    let capture = capture.clone();
    let error = failed.clone();
    let overflow = failed.clone();
    let count = count.clone();
    let level = level.clone();
    device.build_input_stream(
        config,
        move |samples: &[T], _| {
            // Never wait on the control thread. A missed callback makes this recording fail.
            if let Ok(mut audio) = capture.try_lock() {
                audio.push(samples);
                count.store(audio.pcm.len(), Ordering::Release);
                // Report a perceptual RMS level from this callback, without exposing audio.
                let energy = samples
                    .iter()
                    .map(|s| {
                        let value = f32::from_sample(*s);
                        if value.is_finite() {
                            f64::from(value.clamp(-1.0, 1.0)).powi(2)
                        } else {
                            0.0
                        }
                    })
                    .sum::<f64>()
                    / samples.len().max(1) as f64;
                level.store(
                    (energy.sqrt().sqrt() * 100.0).clamp(0.0, 100.0) as u32,
                    Ordering::Release,
                );
            } else {
                overflow.store(true, Ordering::Release);
            }
        },
        move |_| {
            error.store(true, Ordering::Release);
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_formats_channels_and_callback_boundaries() {
        let mut capture = Capture::new(2, 48_000).unwrap();
        for _ in 0..10 {
            capture.push(&vec![16_384i16; 9_600]);
        }
        assert_eq!(capture.pcm.len(), RATE);
        assert!((capture.pcm[100] - 0.5).abs() < 0.001);
        let mut stereo = Capture::new(2, 16_000).unwrap();
        stereo.push(&[0.5f32]);
        stereo.push(&[-0.5f32, 1.0, 1.0]);
        assert_eq!(stereo.pcm, [0.0, 1.0]);
        let mut unsigned = Capture::new(1, 16_000).unwrap();
        unsigned.push(&[32_768u16; 16]);
        assert!(unsigned.pcm.iter().all(|s| s.abs() < 0.001));
    }
    #[test]
    fn bounded_at_sixty_seconds_and_bad_samples_are_safe() {
        let mut capture = Capture::new(1, 16_000).unwrap();
        capture.push(&vec![0.5f32; MAX_SAMPLES + RATE]);
        assert_eq!(capture.pcm.len(), MAX_SAMPLES);
        assert!(capture.full);
        capture.push(&[0.9]);
        assert_eq!(capture.pcm.len(), MAX_SAMPLES);
        assert!(Capture::new(0, 48_000).is_err());
        assert!(Capture::new(2, 0).is_err());
    }
    /// Repeatable noise between -1 and 1, without a dependency.
    fn hiss(n: usize, seed: u32) -> Vec<f32> {
        let mut x = seed;
        (0..n).map(|_| { x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223); (x >> 8) as f32 / (1 << 23) as f32 - 1.0 }).collect()
    }

    /// A recording as people make one: room noise, then words, then room noise.
    fn spoken(noise: f32, voice: f32) -> Vec<f32> {
        let mut pcm: Vec<f32> = hiss(RATE * 3 / 10, 1).into_iter().map(|v| v * noise).collect();
        pcm.extend(hiss(RATE * 6 / 10, 2).into_iter().enumerate().map(|(i, v)| v * noise + (i as f32 * 0.1).sin() * voice));
        pcm.extend(hiss(RATE * 3 / 10, 3).into_iter().map(|v| v * noise));
        pcm
    }

    #[test]
    fn short_silence_and_single_clicks_are_rejected() {
        assert!(speech(&[0.5; 100]).is_err());
        assert!(speech(&vec![0.0; RATE]).is_err());
        let mut click = vec![0.0; RATE];
        click[100] = 1.0;
        assert!(speech(&click).is_err());
        assert!(speech(&spoken(0.0005, 0.06)).is_ok());
        // A quiet microphone previously failed the fixed 0.008 RMS cutoff.
        let quiet = spoken(0.0005, 0.005);
        assert!(speech(&quiet).is_ok());
        assert!(speech(&vec![0.001; RATE]).is_err());
        let mut boosted = quiet;
        normalize(&mut boosted);
        assert!(boosted.iter().any(|v| v.abs() > 0.09));
        assert!(boosted.iter().all(|v| v.abs() <= 1.0));
    }

    #[test]
    fn steady_room_noise_is_not_speech() {
        // Loud enough to pass the fixed floor on its own, but nothing stands out of it.
        let fan: Vec<f32> = hiss(RATE * 2, 4).into_iter().map(|v| v * 0.01).collect();
        assert!(speech(&fan).is_err());
        // Words over the same noise still count.
        assert!(speech(&spoken(0.01, 0.08)).is_ok());
    }

    #[test]
    #[ignore = "Enumerates this machine's input devices without recording"]
    fn list_native_inputs_without_recording() {
        let inputs = inputs().unwrap();
        println!(
            "default: {:?}; microphones: {:?}",
            inputs.default, inputs.devices
        );
    }
}
