use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};

use super::buffer::AudioBuffer;

#[derive(Clone)]
pub struct PlaybackController {
    pub current_sample: Arc<AtomicUsize>,
    pub is_playing: Arc<AtomicBool>,
    pub is_looping: Arc<AtomicBool>,
    pub loop_range: Arc<RwLock<Option<Range<usize>>>>,
    pub meter_l: Arc<AtomicU32>,
    pub meter_r: Arc<AtomicU32>,
    pub master_volume: Arc<AtomicU32>,
    pub buffer: Arc<RwLock<AudioBuffer>>,
}

impl PlaybackController {
    pub fn new(initial_buffer: AudioBuffer) -> Self {
        Self {
            current_sample: Arc::new(AtomicUsize::new(0)),
            is_playing: Arc::new(AtomicBool::new(false)),
            is_looping: Arc::new(AtomicBool::new(false)),
            loop_range: Arc::new(RwLock::new(None)),
            meter_l: Arc::new(AtomicU32::new(0)),
            meter_r: Arc::new(AtomicU32::new(0)),
            master_volume: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            buffer: Arc::new(RwLock::new(initial_buffer)),
        }
    }

    pub fn play(&self) {
        self.is_playing.store(true, Ordering::SeqCst);
    }

    pub fn pause(&self) {
        self.is_playing.store(false, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.is_playing.store(false, Ordering::SeqCst);
        let loop_start = self.loop_range.read().ok().and_then(|r| r.as_ref().map(|x| x.start));
        self.current_sample.store(loop_start.unwrap_or(0), Ordering::SeqCst);
    }

    pub fn toggle_play(&self) {
        let currently = self.is_playing.load(Ordering::SeqCst);
        self.is_playing.store(!currently, Ordering::SeqCst);
    }

    pub fn seek(&self, sample: usize) {
        let max_samples = self.buffer.read().map(|b| b.num_samples()).unwrap_or(0);
        let clamped = sample.min(max_samples);
        self.current_sample.store(clamped, Ordering::SeqCst);
    }

    pub fn set_loop_range(&self, range: Option<Range<usize>>) {
        if let Ok(mut r) = self.loop_range.write() {
            *r = range;
        }
    }

    pub fn set_looping(&self, looping: bool) {
        self.is_looping.store(looping, Ordering::SeqCst);
    }

    pub fn get_peaks(&self) -> (f32, f32) {
        let l = f32::from_bits(self.meter_l.load(Ordering::Relaxed));
        let r = f32::from_bits(self.meter_r.load(Ordering::Relaxed));
        (l, r)
    }

    pub fn set_volume(&self, vol: f32) {
        self.master_volume.store(vol.clamp(0.0, 2.0).to_bits(), Ordering::Relaxed);
    }

    pub fn get_volume(&self) -> f32 {
        f32::from_bits(self.master_volume.load(Ordering::Relaxed))
    }

    pub fn replace_buffer(&self, new_buf: AudioBuffer) {
        let mut buf_guard = self.buffer.write().unwrap();
        *buf_guard = new_buf;
        self.current_sample.store(0, Ordering::SeqCst);
    }
}

pub struct AudioPlaybackEngine {
    pub controller: PlaybackController,
    _stream: Stream,
}

impl AudioPlaybackEngine {
    pub fn try_new(initial_buffer: AudioBuffer) -> anyhow::Result<Self> {
        let controller = PlaybackController::new(initial_buffer);
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;

        let supported_config = device.default_output_config()?;
        let sample_rate = supported_config.sample_rate().0;
        let channels = supported_config.channels();

        let ctrl_clone = controller.clone();

        let err_fn = |err| eprintln!("Audio stream error: {}", err);

        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => device.build_output_stream(
                &supported_config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    write_audio_data(data, channels as usize, sample_rate, &ctrl_clone);
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => device.build_output_stream(
                &supported_config.config(),
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let mut temp = vec![0.0f32; data.len()];
                    write_audio_data(&mut temp, channels as usize, sample_rate, &ctrl_clone);
                    for (out, &sample) in data.iter_mut().zip(temp.iter()) {
                        *out = (sample * 32767.0) as i16;
                    }
                },
                err_fn,
                None,
            )?,
            SampleFormat::U16 => device.build_output_stream(
                &supported_config.config(),
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let mut temp = vec![0.0f32; data.len()];
                    write_audio_data(&mut temp, channels as usize, sample_rate, &ctrl_clone);
                    for (out, &sample) in data.iter_mut().zip(temp.iter()) {
                        *out = ((sample * 0.5 + 0.5) * 65535.0) as u16;
                    }
                },
                err_fn,
                None,
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        stream.play()?;

        Ok(Self {
            controller,
            _stream: stream,
        })
    }
}

fn write_audio_data(
    output: &mut [f32],
    device_channels: usize,
    device_sample_rate: u32,
    ctrl: &PlaybackController,
) {
    if !ctrl.is_playing.load(Ordering::Relaxed) {
        output.fill(0.0);
        ctrl.meter_l.store(0.0f32.to_bits(), Ordering::Relaxed);
        ctrl.meter_r.store(0.0f32.to_bits(), Ordering::Relaxed);
        return;
    }

    let buf_guard = match ctrl.buffer.read() {
        Ok(g) => g,
        Err(_) => {
            output.fill(0.0);
            return;
        }
    };

    let total_samples = buf_guard.num_samples();
    let src_rate = buf_guard.sample_rate;
    if total_samples == 0 || src_rate == 0 {
        output.fill(0.0);
        return;
    }

    let rate_ratio = src_rate as f64 / device_sample_rate as f64;
    let mut pos = ctrl.current_sample.load(Ordering::Relaxed);
    let looping = ctrl.is_looping.load(Ordering::Relaxed);
    let loop_range = ctrl.loop_range.read().ok().and_then(|r| r.clone());

    let (loop_start, loop_end) = match loop_range {
        Some(r) if r.end > r.start => (r.start, r.end.min(total_samples)),
        _ => (0, total_samples),
    };

    let volume = f32::from_bits(ctrl.master_volume.load(Ordering::Relaxed));
    let mut max_l = 0.0f32;
    let mut max_r = 0.0f32;

    let frames = output.len() / device_channels;
    let ch0 = &buf_guard.channels[0];
    let ch1 = if buf_guard.channels.len() > 1 {
        &buf_guard.channels[1]
    } else {
        ch0
    };

    for frame in 0..frames {
        if pos >= total_samples || (looping && pos >= loop_end) {
            if looping {
                pos = loop_start;
            } else {
                ctrl.is_playing.store(false, Ordering::Relaxed);
                pos = 0;
                // Zero remaining frames
                let rem_idx = frame * device_channels;
                output[rem_idx..].fill(0.0);
                break;
            }
        }

        let l_sample = ch0.get(pos).copied().unwrap_or(0.0) * volume;
        let r_sample = ch1.get(pos).copied().unwrap_or(0.0) * volume;

        if l_sample.abs() > max_l { max_l = l_sample.abs(); }
        if r_sample.abs() > max_r { max_r = r_sample.abs(); }

        let out_idx = frame * device_channels;
        if device_channels >= 2 {
            output[out_idx] = l_sample;
            output[out_idx + 1] = r_sample;
            for extra in 2..device_channels {
                output[out_idx + extra] = 0.0;
            }
        } else if device_channels == 1 {
            output[out_idx] = (l_sample + r_sample) * 0.5;
        }

        // Advance sample position with resampling ratio
        let next_pos = pos + rate_ratio.round() as usize;
        pos = if next_pos == pos { pos + 1 } else { next_pos };
    }

    ctrl.current_sample.store(pos, Ordering::Relaxed);
    ctrl.meter_l.store(max_l.to_bits(), Ordering::Relaxed);
    ctrl.meter_r.store(max_r.to_bits(), Ordering::Relaxed);
}
