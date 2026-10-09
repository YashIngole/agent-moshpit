//! Bounded conversion on the device callback. No I/O or inference here.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
    // Require at least 200ms of non-silent 20ms windows, not just a single click.
    let audible = pcm
        .chunks(RATE / 50)
        .filter(|chunk| {
            let energy =
                chunk.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / chunk.len() as f64;
            energy.sqrt() > 0.008
        })
        .count();
    if audible < 10 {
        return Err(
            "No clear speech was heard. Check the default microphone and try again.".into(),
        );
    }
    Ok(())
}

pub struct DeviceCapture {
    pub stream: cpal::Stream,
    pub audio: Arc<Mutex<Capture>>,
    pub failed: Arc<AtomicBool>,
    pub count: Arc<AtomicUsize>,
}

pub fn open() -> Result<DeviceCapture, String> {
    let device = cpal::default_host().default_input_device().ok_or(
        "No microphone was found. Connect one and choose it as the system's default input.",
    )?;
    let config = device.default_input_config().map_err(|e| {
        format!("The default microphone is unavailable: {e}. Check system microphone access.")
    })?;
    let capture = Arc::new(Mutex::new(Capture::new(
        config.channels(),
        config.sample_rate().0,
    )?));
    let failed = Arc::new(AtomicBool::new(false));
    let count = Arc::new(AtomicUsize::new(0));
    let stream_config = config.clone().into();
    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => build::<i8>(&device, &stream_config, &capture, &failed, &count),
        cpal::SampleFormat::I16 => build::<i16>(&device, &stream_config, &capture, &failed, &count),
        cpal::SampleFormat::I32 => build::<i32>(&device, &stream_config, &capture, &failed, &count),
        cpal::SampleFormat::I64 => build::<i64>(&device, &stream_config, &capture, &failed, &count),
        cpal::SampleFormat::U8 => build::<u8>(&device, &stream_config, &capture, &failed, &count),
        cpal::SampleFormat::U16 => build::<u16>(&device, &stream_config, &capture, &failed, &count),
        cpal::SampleFormat::U32 => build::<u32>(&device, &stream_config, &capture, &failed, &count),
        cpal::SampleFormat::U64 => build::<u64>(&device, &stream_config, &capture, &failed, &count),
        cpal::SampleFormat::F32 => build::<f32>(&device, &stream_config, &capture, &failed, &count),
        cpal::SampleFormat::F64 => build::<f64>(&device, &stream_config, &capture, &failed, &count),
        _ => return Err("The default microphone's sample format is unsupported. Choose another input device.".into()),
    }.map_err(|e| format!("The microphone could not start: {e}. Check system microphone access and the default input device."))?;
    stream.play().map_err(|e| {
        format!("The microphone could not record: {e}. Check system microphone access.")
    })?;
    Ok(DeviceCapture {
        stream,
        audio: capture,
        failed,
        count,
    })
}

fn build<T: SizedSample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    capture: &Arc<Mutex<Capture>>,
    failed: &Arc<AtomicBool>,
    count: &Arc<AtomicUsize>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    f32: FromSample<T>,
{
    let capture = capture.clone();
    let error = failed.clone();
    let overflow = failed.clone();
    let count = count.clone();
    device.build_input_stream(
        config,
        move |samples: &[T], _| {
            // Never wait on the control thread. A missed callback makes this recording fail.
            if let Ok(mut audio) = capture.try_lock() {
                audio.push(samples);
                count.store(audio.pcm.len(), Ordering::Release);
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
    #[test]
    fn short_silence_and_single_clicks_are_rejected() {
        assert!(speech(&[0.5; 100]).is_err());
        assert!(speech(&vec![0.0; RATE]).is_err());
        let mut click = vec![0.0; RATE];
        click[100] = 1.0;
        assert!(speech(&click).is_err());
        assert!(speech(&vec![0.04; RATE]).is_ok());
    }
}
