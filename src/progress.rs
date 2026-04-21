use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

impl Verbosity {
    pub fn from_flags(quiet: bool, verbose: bool) -> Self {
        match (quiet, verbose) {
            (true, _) => Verbosity::Quiet,
            (false, true) => Verbosity::Verbose,
            _ => Verbosity::Normal,
        }
    }
}

pub struct Reporter {
    verbosity: Verbosity,
    is_tty: bool,
}

impl Reporter {
    pub fn new(verbosity: Verbosity) -> Self {
        Self {
            verbosity,
            is_tty: std::io::stderr().is_terminal(),
        }
    }

    pub fn spinner(&self, message: &'static str) -> Option<ProgressBar> {
        // Non-TTY callers get a single terminal line per step, emitted by
        // `finish` or `fail`. That keeps logs clean and makes success/failure
        // unambiguous in CI output.
        if self.verbosity == Verbosity::Quiet || !self.is_tty {
            return None;
        }
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message(message);
        Some(pb)
    }

    pub fn download_bar(&self, total: Option<u64>) -> Option<ProgressBar> {
        if self.verbosity == Verbosity::Quiet || !self.is_tty {
            return None;
        }
        let pb = match total {
            Some(t) => ProgressBar::new(t).with_style(
                ProgressStyle::with_template(
                    "{spinner:.cyan} {msg} [{bar:30.cyan}] {bytes}/{total_bytes} ({eta})",
                )
                .unwrap()
                .progress_chars("=>-"),
            ),
            None => ProgressBar::new_spinner()
                .with_style(ProgressStyle::with_template("{spinner:.cyan} {msg} {bytes}").unwrap()),
        };
        pb.set_message("Downloading model");
        Some(pb)
    }

    pub fn finish(&self, pb: Option<ProgressBar>, message: String) {
        if let Some(pb) = pb {
            pb.finish_with_message(message);
        } else if self.verbosity != Verbosity::Quiet {
            eprintln!("{message}");
        }
    }

    /// Replace a live spinner with a failure marker, or emit the line in
    /// non-TTY mode. Callers should prefix the message with `✗ `.
    pub fn fail(&self, pb: Option<ProgressBar>, message: String) {
        if let Some(pb) = pb {
            pb.abandon_with_message(message);
        } else if self.verbosity != Verbosity::Quiet {
            eprintln!("{message}");
        }
    }

    pub fn done(&self, total: Duration) {
        if self.verbosity == Verbosity::Quiet {
            return;
        }
        eprintln!(
            "✓ Done ({})",
            HumanBytes(0)
                .to_string()
                .replace("0 B", &format_duration(total))
        );
    }
}

fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    let m = total / 60;
    let s = total % 60;
    if m > 0 {
        format!("{m}m{s:02}s")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbosity_priority_order() {
        assert_eq!(Verbosity::from_flags(true, true), Verbosity::Quiet);
        assert_eq!(Verbosity::from_flags(false, true), Verbosity::Verbose);
        assert_eq!(Verbosity::from_flags(false, false), Verbosity::Normal);
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(format_duration(Duration::from_secs(5)), "5s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m05s");
    }
}
