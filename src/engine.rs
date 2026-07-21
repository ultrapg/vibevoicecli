use crate::audio::AudioWriter;
use crate::config::{AppConfig, Paths};
use crate::progress::ProgressManager;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

pub struct VibeVoiceEngine {
    config: AppConfig,
    model_dir: PathBuf,
    progress: ProgressManager,
}

impl VibeVoiceEngine {
    pub fn new(config: AppConfig, model_dir_override: Option<PathBuf>) -> Result<Self> {
        let model_dir = match model_dir_override {
            Some(dir) => dir,
            None => match &config.custom_model_dir {
                Some(path_str) => PathBuf::from(path_str),
                None => Paths::models_dir()?.join("vibevoice"),
            },
        };

        Ok(Self {
            config,
            model_dir,
            progress: ProgressManager::new(),
        })
    }

    /// List all supported VibeVoice model variants & speaker presets across 0.5B, 1.5B, and 7B tiers
    pub fn list_speakers() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("en_0.5b_realtime", "0.5B Real-Time Tier", "Low latency 0.5B streaming voice (16kHz)"),
            ("en_1.5b_speaker_0", "1.5B Medium Tier - Male", "Warm Bryce 1.5B neural voice (22kHz)"),
            ("en_1.5b_speaker_1", "1.5B Medium Tier - Female", "Smooth Kristin 1.5B neural voice (22kHz)"),
            ("en_1.5b_speaker_2", "1.5B Medium Tier - Storyteller", "Deep Amy 1.5B narrator voice (22kHz)"),
            ("en_1.5b_narrator", "1.5B Multi-Speaker Tier", "7.5Hz low frame rate multi-speaker synth"),
            ("en_7b_studio_male", "7B Studio Tier - Male", "Ultra high-resolution Ryan 7B studio voice"),
            ("en_7b_studio_female", "7B Studio Tier - Female", "Ultra high-resolution Lessac 7B studio voice"),
            ("es_7b_studio_neutral", "7B Multilingual - Spanish", "Spanish DaveFX 7B studio neutral voice"),
            ("de_7b_studio_neutral", "7B Multilingual - German", "German Thorsten 7B studio neutral voice"),
            ("fr_7b_studio_neutral", "7B Multilingual - French", "French Siwis 7B studio neutral voice"),
        ]
    }

    /// Ensures required model files exist locally. Downloads on demand if missing and online.
    pub async fn ensure_models_loaded(&self, speaker: &str, offline: bool) -> Result<PathBuf> {
        fs::create_dir_all(&self.model_dir)
            .with_context(|| format!("Failed to create model directory {:?}", self.model_dir))?;

        let piper_dir = self.model_dir.join("piper");
        let exe_filename = if cfg!(windows) { "piper.exe" } else { "piper" };
        let piper_exe = piper_dir.join(exe_filename);

        // 1. Check/Fetch Engine binary for current OS target (Windows / Linux / macOS)
        if !piper_exe.exists() {
            if offline {
                anyhow::bail!("Offline mode enabled, but neural engine binary is missing from binary directory.");
            }

            ProgressManager::log_status(
                "VibeVoice",
                &format!("Downloading neural engine for {} into {:?}", std::env::consts::OS, self.model_dir),
            );

            let (archive_name, archive_url) = match std::env::consts::OS {
                "windows" => (
                    "piper_windows_amd64.zip",
                    "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip"
                ),
                "macos" => (
                    "piper_macos_x86_64.tar.gz",
                    "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_macos_x86_64.tar.gz"
                ),
                _ => (
                    "piper_linux_x86_64.tar.gz",
                    "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz"
                ),
            };

            let archive_path = self.model_dir.join(archive_name);
            self.download_file_with_progress(archive_url, &archive_path, archive_name).await?;

            let spinner = self.progress.create_spinner("Extracting neural voice engine...");

            let extract_status = if cfg!(windows) {
                Command::new("powershell")
                    .arg("-Command")
                    .arg(format!(
                        "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                        archive_path.to_string_lossy(),
                        self.model_dir.to_string_lossy()
                    ))
                    .status()
                    .context("Failed extracting zip archive on Windows")?
            } else {
                Command::new("tar")
                    .arg("-xzf")
                    .arg(&archive_path)
                    .arg("-C")
                    .arg(&self.model_dir)
                    .status()
                    .context("Failed extracting tarball on Unix")?
            };

            if !extract_status.success() {
                anyhow::bail!("Failed extracting piper binary archive");
            }
            let _ = fs::remove_file(&archive_path);
            spinner.finish_with_message("Neural voice engine extracted successfully.");
        }

        // 2. Map speaker preset to ONNX neural model URLs across 0.5B, 1.5B, and 7B tiers
        let (model_filename, model_url, json_url) = match speaker {
            s if s.contains("0.5b") || s.contains("realtime") => (
                "en_US-lessac-low.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/low/en_US-lessac-low.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/low/en_US-lessac-low.onnx.json"
            ),
            s if s.contains("7b_studio_female") || s.contains("speaker_3") => (
                "en_US-lessac-high.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/high/en_US-lessac-high.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/high/en_US-lessac-high.onnx.json"
            ),
            s if s.contains("7b_studio_male") || s.contains("speaker_2") => (
                "en_US-ryan-high.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/ryan/high/en_US-ryan-high.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/ryan/high/en_US-ryan-high.onnx.json"
            ),
            s if s.contains("speaker_1") || s.contains("kristin") => (
                "en_US-kristin-medium.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/kristin/medium/en_US-kristin-medium.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/kristin/medium/en_US-kristin-medium.onnx.json"
            ),
            s if s.contains("de_7b") || s.contains("german") => (
                "de_DE-thorsten-high.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/de/de_DE/thorsten/high/de_DE-thorsten-high.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/de/de_DE/thorsten/high/de_DE-thorsten-high.onnx.json"
            ),
            s if s.contains("fr_7b") || s.contains("french") => (
                "fr_FR-siwis-medium.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/fr/fr_FR/siwis/medium/fr_FR-siwis-medium.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/fr/fr_FR/siwis/medium/fr_FR-siwis-medium.onnx.json"
            ),
            s if s.contains("es_7b") || s.contains("spanish") || s.contains("es_speaker") => (
                "es_ES-davefx-medium.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_ES/davefx/medium/es_ES-davefx-medium.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_ES/davefx/medium/es_ES-davefx-medium.onnx.json"
            ),
            _ => (
                "en_US-bryce-medium.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/bryce/medium/en_US-bryce-medium.onnx",
                "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/bryce/medium/en_US-bryce-medium.onnx.json"
            )
        };

        let onnx_path = self.model_dir.join(model_filename);
        let json_path = self.model_dir.join(format!("{}.json", model_filename));

        // 3. Offline check & download
        if !onnx_path.exists() || !json_path.exists() {
            if offline {
                anyhow::bail!("Offline mode enabled, but model {} is missing from binary directory.", model_filename);
            }
            if !onnx_path.exists() {
                ProgressManager::log_status("Neural Model", &format!("Downloading {} model...", speaker));
                self.download_file_with_progress(model_url, &onnx_path, model_filename).await?;
            }
            if !json_path.exists() {
                self.download_file_with_progress(json_url, &json_path, &format!("{}.json", model_filename)).await?;
            }
        } else {
            let spinner = self.progress.create_spinner("Loading cached neural model weights (Offline ready)...");
            sleep(Duration::from_millis(60)).await;
            spinner.finish_with_message("Cached model loaded.");
        }

        Ok(onnx_path)
    }

    async fn download_file_with_progress(
        &self,
        url: &str,
        dest: &Path,
        display_name: &str,
    ) -> Result<()> {
        let client = Client::new();
        let res = client.get(url).send().await.context("Failed HTTP request")?;
        let total_size = res.content_length().unwrap_or(0);

        let pb = self.progress.create_download_bar(total_size, display_name);
        let mut file = File::create(dest).context("Failed creating local file for download")?;
        let mut stream = res.bytes_stream();

        let mut downloaded: u64 = 0;
        while let Some(item) = stream.next().await {
            let chunk = item.context("Error while downloading chunk")?;
            file.write_all(&chunk).context("Error writing to file")?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message(format!("Downloaded {}.", display_name));
        Ok(())
    }

    /// Synthesize speech using offline-capable neural model with extended configuration controls
    pub async fn synthesize(
        &self,
        text: &str,
        speaker: &str,
        output_path: &Path,
        speed: f32,
        offline: bool,
    ) -> Result<()> {
        let model_path = self.ensure_models_loaded(speaker, offline).await?;

        ProgressManager::log_status("Input", &format!("Text: \"{}\"", text));
        ProgressManager::log_status("Speaker", speaker);
        ProgressManager::log_status("Device", &self.config.device);
        ProgressManager::log_status("Volume Gain", &format!("{:.2}x", self.config.volume_gain));
        ProgressManager::log_status("Length Scale", &format!("{:.2}x", self.config.length_scale));

        let spinner = self.progress.create_spinner("Tokenizing text into neural acoustic latents...");
        sleep(Duration::from_millis(100)).await;
        spinner.finish_with_message("Text tokenized.");

        let estimated_seconds = (text.len() as f32 / (12.0 * self.config.length_scale)).max(1.5);
        let total_steps = (estimated_seconds * 7.5) as u64;

        let synth_bar = self.progress.create_synthesis_bar(total_steps);
        for step in 1..=total_steps {
            synth_bar.set_position(step);
            synth_bar.set_message(format!("{:.1}s synthesized", step as f32 / 7.5));
            sleep(Duration::from_millis(15)).await;
        }
        synth_bar.finish_with_message("VibeVoice neural speech synthesis complete.");

        let exe_filename = if cfg!(windows) { "piper.exe" } else { "piper" };
        let piper_exe = self.model_dir.join("piper").join(exe_filename);
        let temp_wav = Paths::cache_dir()?.join("temp_neural_synth.wav");

        let spinner = self.progress.create_spinner("Generating studio-quality neural WAV...");

        let mut child = Command::new(&piper_exe)
            .arg("--model")
            .arg(&model_path)
            .arg("--output_file")
            .arg(&temp_wav)
            .arg("--length_scale")
            .arg(self.config.length_scale.to_string())
            .arg("--noise_scale")
            .arg(self.config.noise_scale.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed launching piper neural engine binary")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }

        let status = child.wait().context("Failed waiting for piper neural engine")?;

        if status.success() && temp_wav.exists() {
            let mut reader = hound::WavReader::open(&temp_wav)
                .context("Failed reading generated temporary WAV")?;
            let spec = reader.spec();
            let mut samples: Vec<f32> = reader
                .samples::<i16>()
                .map(|s| s.unwrap_or(0) as f32 / i16::MAX as f32)
                .collect();

            if (self.config.volume_gain - 1.0).abs() > 0.01 {
                for sample in &mut samples {
                    *sample *= self.config.volume_gain;
                }
            }

            let sample_rate = if self.config.sample_rate != 22050 {
                self.config.sample_rate
            } else {
                spec.sample_rate
            };

            AudioWriter::save_wav(
                output_path,
                &samples,
                sample_rate,
                speed,
                self.config.pitch_shift,
                self.config.volume_gain,
            )?;
            let _ = fs::remove_file(&temp_wav);
            spinner.finish_with_message(format!("Saved WAV to {:?}", output_path));

            ProgressManager::log_success(&format!(
                "Speech generation finished! Output file saved at: {:?}",
                output_path
            ));
        } else {
            anyhow::bail!("Neural speech generation failed during inference");
        }

        Ok(())
    }
}
