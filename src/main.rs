mod audio;
mod cli;
mod config;
mod engine;
mod progress;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use config::Paths;
use engine::VibeVoiceEngine;
use progress::ProgressManager;
use std::fs;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt::init();
    }

    // 1. Handle --clean feature
    if cli.clean {
        ProgressManager::log_status("Clean", "Deleting all models, cache, and configuration in binary directory...");
        let (bytes, count) = Paths::clean_all().context("Failed performing clean operation")?;
        let mb = bytes as f64 / (1024.0 * 1024.0);
        ProgressManager::log_success(&format!(
            "Cleanup complete! Removed {} files ({:.2} MB) from binary directory.",
            count, mb
        ));
        return Ok(());
    }

    // 2. Initialize strictly portable binary directory environment
    Paths::init_environment().context("Failed to initialize portable directory structure")?;

    let mut config = Paths::load_or_create_config()?;

    // 3. Handle --show-config
    if cli.show_config {
        println!("\nCurrent VibeVoice CLI Configuration (<binary_dir>/config.json):\n");
        let json_str = serde_json::to_string_pretty(&config)?;
        println!("{}", json_str);
        println!();
        return Ok(());
    }

    // 4. Handle --set-config key=value
    if let Some(pair) = &cli.set_config {
        let parts: Vec<&str> = pair.splitn(2, '=').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid format for --set-config. Use format: key=value (e.g. default_speaker=en_7b_studio_male)");
        }
        let key = parts[0].trim();
        let val = parts[1].trim();

        match key {
            "default_speaker" => config.default_speaker = val.to_string(),
            "device" => config.device = val.to_string(),
            "default_speed" => config.default_speed = val.parse()?,
            "sample_rate" => config.sample_rate = val.parse()?,
            "volume_gain" => config.volume_gain = val.parse()?,
            "pitch_shift" => config.pitch_shift = val.parse()?,
            "length_scale" => config.length_scale = val.parse()?,
            "noise_scale" => config.noise_scale = val.parse()?,
            "output_format" => config.output_format = val.to_string(),
            "cache_enabled" => config.cache_enabled = val.parse()?,
            "custom_model_dir" => config.custom_model_dir = Some(val.to_string()),
            _ => anyhow::bail!("Unknown configuration key '{}'", key),
        }

        let cfg_path = Paths::config_file()?;
        let json_str = serde_json::to_string_pretty(&config)?;
        fs::write(&cfg_path, json_str)?;
        ProgressManager::log_success(&format!("Configuration updated: {} = {}", key, val));
        return Ok(());
    }

    // 5. Handle --list-speakers
    if cli.list_speakers {
        println!("\nAvailable VibeVoice Speaker Presets & Model Tiers (0.5B, 1.5B, 7B):\n");
        println!("{:<24} {:<30} DESCRIPTION", "PRESET NAME", "MODEL TIER / TYPE");
        println!("{}", "-".repeat(80));
        for (id, type_str, desc) in VibeVoiceEngine::list_speakers() {
            println!("{:<24} {:<30} {}", id, type_str, desc);
        }
        println!();
        return Ok(());
    }

    // 6. Apply CLI argument overrides to config
    if let Some(d) = cli.device { config.device = d; }
    if let Some(v) = cli.volume { config.volume_gain = v; }
    if let Some(p) = cli.pitch { config.pitch_shift = p; }
    if let Some(l) = cli.length_scale { config.length_scale = l; }
    if let Some(n) = cli.noise_scale { config.noise_scale = n; }
    if let Some(sr) = cli.sample_rate { config.sample_rate = sr; }

    let speed = cli.speed.unwrap_or(config.default_speed);
    let speaker = cli
        .speaker
        .clone()
        .unwrap_or(config.default_speaker.clone());

    let output_path = match cli.output {
        Some(path) => path,
        None => Paths::default_output()?,
    };

    let engine = VibeVoiceEngine::new(config, cli.model_dir)?;

    // 7. Handle Interactive mode
    if cli.interactive {
        println!("=== VibeVoice CLI Interactive Mode ===");
        println!("Type text to synthesize speech, or 'exit' to quit.\n");
        let stdin = io::stdin();
        let mut reader = stdin.lock();

        let mut count = 1;
        loop {
            print!("vibevoice> ");
            io::stdout().flush()?;
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
                break;
            }

            let interactive_out = Paths::working_dir()?.join(format!("output_{}.wav", count));
            engine
                .synthesize(trimmed, &speaker, &interactive_out, speed, cli.offline)
                .await?;
            count += 1;
        }
        return Ok(());
    }

    // 8. Single text synthesis execution
    let input_text = if let Some(t) = cli.text {
        t
    } else if let Some(file_path) = cli.file {
        fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read input file {:?}", file_path))?
    } else {
        println!("No text input provided. Generating default sample speech...");
        "Welcome to VibeVoice CLI. A standalone and portable voice text-to-speech synthesis tool written in Rust.".to_string()
    };

    engine
        .synthesize(&input_text, &speaker, &output_path, speed, cli.offline)
        .await?;

    Ok(())
}
