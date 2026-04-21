//! Logging setup: redirects whisper.cpp / ggml native stderr writes through
//! the `log` crate so they can be filtered by verbosity. Without this, the
//! C library dumps ~45 lines of init output every transcription, ignoring the
//! `-q` flag.

use log::{LevelFilter, Metadata, Record};
use std::io::Write;

/// Install whisper-rs's C-side logging hooks and, if `verbose` is set, a
/// minimal stderr logger that prints at Debug level. In non-verbose mode no
/// logger is registered, so records are silently dropped.
pub fn init(verbose: bool) {
    whisper_rs::install_whisper_log_trampoline();

    if !verbose {
        return;
    }

    if log::set_boxed_logger(Box::new(StderrLogger)).is_ok() {
        log::set_max_level(LevelFilter::Debug);
    }
}

struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let _ = writeln!(
            std::io::stderr(),
            "[{} {}] {}",
            record.level(),
            record.target(),
            record.args()
        );
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}
