use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "vibevoicecli",
    author = "vibevoicecli team",
    version = "0.1.0",
    about = "Standalone & portable VibeVoice TTS CLI tool",
    long_about = "vibevoicecli synthesizes speech using VibeVoice models while storing all model files and cache strictly in the executable directory."
)]
pub struct Cli {
    /// Text to synthesize into speech
    #[arg(short = 't', long = "text")]
    pub text: Option<String>,

    /// Read text input from specified file path
    #[arg(short = 'f', long = "file")]
    pub file: Option<PathBuf>,

    /// Output WAV audio file path (defaults to current working directory output.wav)
    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    /// Speaker voice preset or model variant (e.g. 'en_0.5b_realtime', 'en_1.5b_speaker_0', 'en_1.5b_speaker_1', 'en_7b_studio_male', 'en_7b_studio_female', 'es_7b_studio_neutral', 'de_7b_studio_neutral', 'fr_7b_studio_neutral')
    #[arg(short = 's', long = "speaker")]
    pub speaker: Option<String>,

    /// Model quality tier: 'low', 'medium', 'high', 'realtime', or 'studio'
    #[arg(short = 'q', long = "quality")]
    pub quality: Option<String>,

    /// Target execution device: 'auto', 'cpu', or 'cuda'
    #[arg(short = 'd', long = "device")]
    pub device: Option<String>,

    /// Speech generation speed multiplier (default: 1.0)
    #[arg(long = "speed")]
    pub speed: Option<f32>,

    /// Audio volume gain multiplier (default: 1.0)
    #[arg(long = "volume")]
    pub volume: Option<f32>,

    /// Pitch shift multiplier (default: 1.0)
    #[arg(long = "pitch")]
    pub pitch: Option<f32>,

    /// Speaking pace / phoneme length scale (default: 1.0)
    #[arg(long = "length-scale")]
    pub length_scale: Option<f32>,

    /// Phoneme noise scale variability (default: 0.667)
    #[arg(long = "noise-scale")]
    pub noise_scale: Option<f32>,

    /// Output sample rate in Hz (16000, 22050, 24000, 44100, 48000)
    #[arg(long = "sample-rate")]
    pub sample_rate: Option<u32>,

    /// Directory for model storage (defaults to executable's directory 'models')
    #[arg(short = 'm', long = "model-dir")]
    pub model_dir: Option<PathBuf>,

    /// Force offline mode (do not attempt network requests)
    #[arg(long = "offline")]
    pub offline: bool,

    /// Delete all downloaded model files, cache, and temporary data inside binary directory
    #[arg(long = "clean")]
    pub clean: bool,

    /// Display current active configuration settings
    #[arg(long = "show-config")]
    pub show_config: bool,

    /// Set a default configuration value (e.g. --set-config default_speaker=en_7b_studio_male)
    #[arg(long = "set-config")]
    pub set_config: Option<String>,

    /// List available speaker voice presets and VibeVoice model tiers
    #[arg(long = "list-speakers")]
    pub list_speakers: bool,

    /// Launch interactive text input loop
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// Enable verbose diagnostic logging
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}
