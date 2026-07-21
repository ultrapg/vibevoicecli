use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct ProgressManager {
    mp: MultiProgress,
}

impl ProgressManager {
    pub fn new() -> Self {
        Self {
            mp: MultiProgress::new(),
        }
    }

    /// Create an animated spinner for initial setup and model loading steps.
    pub fn create_spinner(&self, message: &'static str) -> ProgressBar {
        let pb = self.mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.cyan} {msg}")
                .expect("Failed to parse spinner template"),
        );
        pb.set_message(message);
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    }

    /// Create a progress bar for downloading model files.
    pub fn create_download_bar(&self, total_bytes: u64, name: &str) -> ProgressBar {
        let pb = self.mp.add(ProgressBar::new(total_bytes));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.bold.dim} [{elapsed_precise}] [{bar:30.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
                .expect("Failed to parse download bar template")
                .progress_chars("#>-"),
        );
        pb.set_prefix(format!("Download {}", name));
        pb
    }

    /// Create a progress bar for acoustic/speech token synthesis.
    pub fn create_synthesis_bar(&self, total_steps: u64) -> ProgressBar {
        let pb = self.mp.add(ProgressBar::new(total_steps));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.bold.green} [{elapsed_precise}] [{bar:35.magenta/cyan}] {pos}/{len} steps ({eta}) {msg}")
                .expect("Failed to parse synthesis bar template")
                .progress_chars("━➤━"),
        );
        pb.set_prefix("Synthesizing");
        pb
    }

    /// Print styled status line.
    pub fn log_status(tag: &str, msg: &str) {
        println!("[{}] {}", style(tag).bold().cyan(), msg);
    }

    /// Print styled success line.
    pub fn log_success(msg: &str) {
        println!("{} {}", style("✔").bold().green(), msg);
    }
}
