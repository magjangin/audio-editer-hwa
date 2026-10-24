use std::ops::Range;

#[derive(Clone, Debug)]
pub struct PeakSample {
    pub min: f32,
    pub max: f32,
    pub rms: f32,
}

#[derive(Clone, Debug)]
pub struct MipmapLevel {
    pub block_size: usize,
    /// Channel-indexed peaks: channels[ch][block_idx]
    pub peaks: Vec<Vec<PeakSample>>,
}

#[derive(Clone, Debug)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    /// channels[channel_index][sample_index] in range -1.0 ..= 1.0
    pub channels: Vec<Vec<f32>>,
    pub mipmaps: Vec<MipmapLevel>,
}

impl AudioBuffer {
    pub const MIPMAP_FACTORS: &'static [usize] = &[64, 256, 1024, 4096];

    pub fn new(sample_rate: u32, channels: Vec<Vec<f32>>) -> Self {
        let mut buffer = Self {
            sample_rate,
            channels,
            mipmaps: Vec::new(),
        };
        buffer.rebuild_mipmaps();
        buffer
    }

    pub fn empty(sample_rate: u32, num_channels: usize) -> Self {
        let channels = (0..num_channels).map(|_| Vec::new()).collect();
        Self {
            sample_rate,
            channels,
            mipmaps: Vec::new(),
        }
    }

    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }

    pub fn num_samples(&self) -> usize {
        self.channels.first().map(|ch| ch.len()).unwrap_or(0)
    }

    pub fn duration_seconds(&self) -> f32 {
        if self.sample_rate == 0 {
            0.0
        } else {
            self.num_samples() as f32 / self.sample_rate as f32
        }
    }

    pub fn sample_to_time(&self, sample: usize) -> f32 {
        if self.sample_rate == 0 {
            0.0
        } else {
            sample as f32 / self.sample_rate as f32
        }
    }

    pub fn time_to_sample(&self, time: f32) -> usize {
        if time <= 0.0 {
            0
        } else {
            ((time * self.sample_rate as f32) as usize).min(self.num_samples())
        }
    }

    pub fn rebuild_mipmaps(&mut self) {
        self.mipmaps.clear();
        let num_ch = self.num_channels();
        if num_ch == 0 || self.num_samples() == 0 {
            return;
        }

        for &factor in Self::MIPMAP_FACTORS {
            let mut ch_peaks = Vec::with_capacity(num_ch);
            for ch in &self.channels {
                let chunks = ch.chunks(factor);
                let mut peaks = Vec::with_capacity(chunks.len());
                for chunk in chunks {
                    let mut min = 0.0f32;
                    let mut max = 0.0f32;
                    let mut sum_sq = 0.0f32;
                    for &s in chunk {
                        if s < min { min = s; }
                        if s > max { max = s; }
                        sum_sq += s * s;
                    }
                    let rms = (sum_sq / chunk.len() as f32).sqrt();
                    peaks.push(PeakSample { min, max, rms });
                }
                ch_peaks.push(peaks);
            }
            self.mipmaps.push(MipmapLevel {
                block_size: factor,
                peaks: ch_peaks,
            });
        }
    }

    /// Query min/max/rms peaks across a range of samples for a given pixel width.
    /// Chooses the optimal mipmap level to achieve O(pixel_width) drawing speed.
    pub fn get_peaks(
        &self,
        channel: usize,
        start_sample: usize,
        end_sample: usize,
        target_buckets: usize,
    ) -> Vec<PeakSample> {
        let ch_data = match self.channels.get(channel) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if target_buckets == 0 || start_sample >= end_sample || start_sample >= ch_data.len() {
            return Vec::new();
        }
        let end_sample = end_sample.min(ch_data.len());
        let span = end_sample - start_sample;
        let samples_per_bucket = span as f32 / target_buckets as f32;

        let mut result = Vec::with_capacity(target_buckets);

        // Find appropriate mipmap level if sample density is high
        let best_level = self.mipmaps.iter().rev().find(|m| {
            (m.block_size as f32) <= samples_per_bucket * 0.75
        });

        if let Some(level) = best_level {
            let ch_peaks = &level.peaks[channel];
            let b_size = level.block_size;
            for i in 0..target_buckets {
                let bucket_start = start_sample + ((i as f32 * samples_per_bucket) as usize);
                let bucket_end = (start_sample + (((i + 1) as f32 * samples_per_bucket) as usize)).min(end_sample);
                if bucket_start >= bucket_end {
                    continue;
                }

                let b_start_idx = bucket_start / b_size;
                let b_end_idx = (bucket_end + b_size - 1) / b_size;
                let b_end_idx = b_end_idx.min(ch_peaks.len());

                let mut min = 0.0f32;
                let mut max = 0.0f32;
                let mut rms_sum = 0.0f32;
                let mut count = 0;

                for p in &ch_peaks[b_start_idx..b_end_idx] {
                    if p.min < min { min = p.min; }
                    if p.max > max { max = p.max; }
                    rms_sum += p.rms * p.rms;
                    count += 1;
                }

                let rms = if count > 0 { (rms_sum / count as f32).sqrt() } else { 0.0 };
                result.push(PeakSample { min, max, rms });
            }
        } else {
            // Direct sample scan for zoomed-in view
            for i in 0..target_buckets {
                let bucket_start = start_sample + ((i as f32 * samples_per_bucket) as usize);
                let bucket_end = (start_sample + (((i + 1) as f32 * samples_per_bucket) as usize)).min(end_sample);
                if bucket_start >= bucket_end {
                    continue;
                }
                let slice = &ch_data[bucket_start..bucket_end];
                let mut min = 0.0f32;
                let mut max = 0.0f32;
                let mut sum_sq = 0.0f32;
                for &s in slice {
                    if s < min { min = s; }
                    if s > max { max = s; }
                    sum_sq += s * s;
                }
                let rms = (sum_sq / slice.len() as f32).sqrt();
                result.push(PeakSample { min, max, rms });
            }
        }

        result
    }

    // --- Audio Editing Operations ---

    pub fn copy_range(&self, range: Range<usize>) -> AudioBuffer {
        let total = self.num_samples();
        let start = range.start.min(total);
        let end = range.end.min(total);
        if start >= end {
            return Self::empty(self.sample_rate, self.num_channels());
        }

        let mut channels = Vec::with_capacity(self.num_channels());
        for ch in &self.channels {
            channels.push(ch[start..end].to_vec());
        }

        Self::new(self.sample_rate, channels)
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        let total = self.num_samples();
        let start = range.start.min(total);
        let end = range.end.min(total);
        if start >= end {
            return;
        }

        for ch in &mut self.channels {
            ch.drain(start..end);
        }
        self.rebuild_mipmaps();
    }

    pub fn cut_range(&mut self, range: Range<usize>) -> AudioBuffer {
        let copied = self.copy_range(range.clone());
        self.delete_range(range);
        copied
    }

    pub fn paste_at(&mut self, position: usize, clip: &AudioBuffer) {
        if clip.num_samples() == 0 || self.num_channels() == 0 {
            return;
        }
        let pos = position.min(self.num_samples());
        for ch_idx in 0..self.num_channels() {
            let clip_ch = clip.channels.get(ch_idx).or_else(|| clip.channels.first());
            if let Some(src) = clip_ch {
                self.channels[ch_idx].splice(pos..pos, src.iter().copied());
            }
        }
        self.rebuild_mipmaps();
    }

    pub fn silence_range(&mut self, range: Range<usize>) {
        let total = self.num_samples();
        let start = range.start.min(total);
        let end = range.end.min(total);
        if start >= end {
            return;
        }

        for ch in &mut self.channels {
            for s in &mut ch[start..end] {
                *s = 0.0;
            }
        }
        self.rebuild_mipmaps();
    }

    pub fn insert_silence(&mut self, position: usize, count_samples: usize) {
        if count_samples == 0 || self.num_channels() == 0 {
            return;
        }
        let pos = position.min(self.num_samples());
        for ch in &mut self.channels {
            ch.splice(pos..pos, std::iter::repeat(0.0f32).take(count_samples));
        }
        self.rebuild_mipmaps();
    }

    pub fn apply_gain_range(&mut self, range: Range<usize>, gain_linear: f32) {
        let total = self.num_samples();
        let start = range.start.min(total);
        let end = range.end.min(total);
        if start >= end {
            return;
        }

        for ch in &mut self.channels {
            for s in &mut ch[start..end] {
                *s = (*s * gain_linear).clamp(-1.0, 1.0);
            }
        }
        self.rebuild_mipmaps();
    }

    pub fn normalize_range(&mut self, range: Range<usize>, target_peak_linear: f32) {
        let total = self.num_samples();
        let start = range.start.min(total);
        let end = range.end.min(total);
        if start >= end {
            return;
        }

        let mut max_abs = 0.0f32;
        for ch in &self.channels {
            for &s in &ch[start..end] {
                let abs = s.abs();
                if abs > max_abs {
                    max_abs = abs;
                }
            }
        }

        if max_abs > 0.00001 {
            let multiplier = target_peak_linear / max_abs;
            self.apply_gain_range(start..end, multiplier);
        }
    }

    pub fn fade(&mut self, range: Range<usize>, fade_in: bool) {
        let total = self.num_samples();
        let start = range.start.min(total);
        let end = range.end.min(total);
        if start >= end {
            return;
        }
        let len = (end - start) as f32;

        for ch in &mut self.channels {
            for (idx, s) in ch[start..end].iter_mut().enumerate() {
                let factor = if fade_in {
                    idx as f32 / len
                } else {
                    1.0 - (idx as f32 / len)
                };
                *s *= factor;
            }
        }
        self.rebuild_mipmaps();
    }

    /// Generates a rich demo track: Stereophonic electronic synth arpeggio + pad + beats
    pub fn generate_demo_track(sample_rate: u32, duration_secs: f32) -> Self {
        let total_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut left = Vec::with_capacity(total_samples);
        let mut right = Vec::with_capacity(total_samples);

        // Chord frequencies (C-minor: C4, Eb4, G4, Bb4)
        let chord = [261.63, 311.13, 392.00, 466.16, 523.25];
        let bpm = 128.0;
        let samples_per_beat = (sample_rate as f32 * (60.0 / bpm)) as usize;
        let step_samples = samples_per_beat / 4; // 16th notes

        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let step = (i / step_samples) % chord.len();
            let note_freq = chord[step];
            let note_t = (i % step_samples) as f32 / sample_rate as f32;

            // Pluck envelope
            let env = (-note_t * 12.0).exp();

            // Synth tone with panning
            let pan = (t * 1.5).sin();
            let left_pan = (1.0 - pan) * 0.5;
            let right_pan = (1.0 + pan) * 0.5;

            let synth = (note_freq * t * 2.0 * std::f32::consts::PI).sin() * env * 0.35;
            // Pad base note
            let pad = (130.81 * t * 2.0 * std::f32::consts::PI).sin() * 0.15;

            // Kick drum on beats
            let beat_idx = i % samples_per_beat;
            let kick_t = beat_idx as f32 / sample_rate as f32;
            let kick_env = (-kick_t * 25.0).exp();
            let kick_freq = 150.0 * (-kick_t * 30.0).exp() + 45.0;
            let kick = (kick_freq * kick_t * 2.0 * std::f32::consts::PI).sin() * kick_env * 0.45;

            let l_sample = (synth * left_pan + pad * 0.8 + kick).clamp(-1.0, 1.0);
            let r_sample = (synth * right_pan + pad * 0.8 + kick).clamp(-1.0, 1.0);

            left.push(l_sample);
            right.push(r_sample);
        }

        Self::new(sample_rate, vec![left, right])
    }
}
