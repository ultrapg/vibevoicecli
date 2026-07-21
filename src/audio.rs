use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

pub struct AudioWriter;

impl AudioWriter {
    /// Save float PCM audio samples (-1.0 to 1.0) into a WAV file with speed, pitch, and tone adjustments.
    pub fn save_wav<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        speed: f32,
        pitch_shift: f32,
        volume_gain: f32,
    ) -> Result<()> {
        let path = path.as_ref();

        // 1. Pitch shift adjustment if specified
        let pitch_processed = if (pitch_shift - 1.0).abs() > 0.01 && pitch_shift > 0.1 {
            Self::adjust_pitch(samples, pitch_shift)
        } else {
            samples.to_vec()
        };

        // 2. Speed / pace adjustment if specified
        let speed_processed = if (speed - 1.0).abs() > 0.01 && speed > 0.1 {
            Self::adjust_speed(&pitch_processed, speed)
        } else {
            pitch_processed
        };

        // 3. Peak volume normalization with custom volume gain
        let max_peak = speed_processed
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, |a, b| a.max(b));
        let norm_factor = if max_peak > 0.001 { (0.90 / max_peak) * volume_gain } else { volume_gain };

        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let file = File::create(path)
            .with_context(|| format!("Failed to create output audio file at {:?}", path))?;
        let writer = BufWriter::new(file);
        let mut wav_writer = WavWriter::new(writer, spec)
            .with_context(|| "Failed to initialize WAV writer")?;

        for &sample in &speed_processed {
            let normalized = (sample * norm_factor).clamp(-0.99, 0.99);
            let sample_i16 = (normalized * i16::MAX as f32) as i16;
            wav_writer
                .write_sample(sample_i16)
                .context("Failed writing audio sample to WAV")?;
        }

        wav_writer.finalize().context("Failed finalizing WAV file")?;
        Ok(())
    }

    /// Pitch shift transformation using linear resampling interpolation.
    fn adjust_pitch(samples: &[f32], pitch_ratio: f32) -> Vec<f32> {
        if samples.is_empty() {
            return vec![];
        }
        let mut res = Vec::with_capacity(samples.len());

        for i in 0..samples.len() {
            let orig_idx = i as f32 * pitch_ratio;
            if orig_idx >= (samples.len() - 1) as f32 {
                break;
            }
            let idx0 = orig_idx.floor() as usize;
            let idx1 = (idx0 + 1).min(samples.len() - 1);
            let frac = orig_idx - idx0 as f32;

            let interpolated = samples[idx0] * (1.0 - frac) + samples[idx1] * frac;
            res.push(interpolated);
        }
        res
    }

    /// Speed / pace transformation using linear interpolation.
    fn adjust_speed(samples: &[f32], speed: f32) -> Vec<f32> {
        if samples.is_empty() {
            return vec![];
        }
        let new_len = (samples.len() as f32 / speed).max(1.0) as usize;
        let mut res = Vec::with_capacity(new_len);

        for i in 0..new_len {
            let orig_idx = i as f32 * speed;
            let idx0 = (orig_idx.floor() as usize).min(samples.len() - 1);
            let idx1 = (idx0 + 1).min(samples.len() - 1);
            let frac = orig_idx - idx0 as f32;

            let interpolated = samples[idx0] * (1.0 - frac) + samples[idx1] * frac;
            res.push(interpolated);
        }
        res
    }
}
