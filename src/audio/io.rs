use std::fs::File;
use std::path::Path;
use anyhow::{anyhow, Result};
use hound::{WavReader, WavSpec, WavWriter};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::buffer::AudioBuffer;

pub fn load_audio_file<P: AsRef<Path>>(path: P) -> Result<AudioBuffer> {
    let path_ref = path.as_ref();
    let ext = path_ref
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Fast-path: Hound WAV reader
    if ext == "wav" {
        if let Ok(buf) = load_wav_hound(path_ref) {
            return Ok(buf);
        }
    }

    // Universal Symphonia loader (MP3, FLAC, OGG, WAV, etc.)
    load_audio_symphonia(path_ref)
}

fn load_wav_hound(path: &Path) -> Result<AudioBuffer> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();
    let num_channels = spec.channels as usize;
    let sample_rate = spec.sample_rate;

    let mut channels: Vec<Vec<f32>> = (0..num_channels).map(|_| Vec::new()).collect();

    match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => {
            let mut ch = 0;
            for sample in reader.samples::<f32>() {
                channels[ch].push(sample?);
                ch = (ch + 1) % num_channels;
            }
        }
        (hound::SampleFormat::Int, 16) => {
            let mut ch = 0;
            for sample in reader.samples::<i16>() {
                channels[ch].push(sample? as f32 / 32768.0);
                ch = (ch + 1) % num_channels;
            }
        }
        (hound::SampleFormat::Int, 24) => {
            let mut ch = 0;
            for sample in reader.samples::<i32>() {
                channels[ch].push(sample? as f32 / 8388608.0);
                ch = (ch + 1) % num_channels;
            }
        }
        (hound::SampleFormat::Int, 32) => {
            let mut ch = 0;
            for sample in reader.samples::<i32>() {
                channels[ch].push(sample? as f32 / 2147483648.0);
                ch = (ch + 1) % num_channels;
            }
        }
        _ => return Err(anyhow!("Unsupported WAV sample format")),
    }

    Ok(AudioBuffer::new(sample_rate, channels))
}

fn load_audio_symphonia(path: &Path) -> Result<AudioBuffer> {
    let src = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .map_err(|e| anyhow!("Failed to probe audio format: {}", e))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("No supported audio track found"))?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("Missing sample rate"))?;
    let channels_count = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(2);

    let dec_opts: DecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .map_err(|e| anyhow!("Failed to create audio decoder: {}", e))?;

    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut channel_data: Vec<Vec<f32>> = (0..channels_count).map(|_| Vec::new()).collect();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(e) => return Err(anyhow!("Packet read error: {}", e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                if sample_buf.is_none() {
                    let spec = *decoded.spec();
                    let duration = decoded.capacity() as u64;
                    sample_buf = Some(SampleBuffer::new(duration, spec));
                }

                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(decoded);
                    let samples = buf.samples();
                    let chs = channels_count;
                    for (i, &s) in samples.iter().enumerate() {
                        let ch_idx = i % chs;
                        if ch_idx < channel_data.len() {
                            channel_data[ch_idx].push(s);
                        }
                    }
                }
            }
            Err(SymphoniaError::IoError(_)) => break,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(anyhow!("Decode failure: {}", e)),
        }
    }

    Ok(AudioBuffer::new(sample_rate, channel_data))
}

pub fn save_wav<P: AsRef<Path>>(path: P, buffer: &AudioBuffer) -> Result<()> {
    let num_channels = buffer.num_channels();
    if num_channels == 0 || buffer.num_samples() == 0 {
        return Err(anyhow!("Cannot save empty audio buffer"));
    }

    let spec = WavSpec {
        channels: num_channels as u16,
        sample_rate: buffer.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;
    let total_samples = buffer.num_samples();

    for i in 0..total_samples {
        for ch in 0..num_channels {
            let s = buffer.channels[ch][i].clamp(-1.0, 1.0);
            let sample_i16 = (s * 32767.0) as i16;
            writer.write_sample(sample_i16)?;
        }
    }

    writer.finalize()?;
    Ok(())
}
