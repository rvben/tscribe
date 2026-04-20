use crate::deps::{self, FFMPEG};
use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Convert any audio file to 16kHz mono PCM s16le WAV (whisper.cpp's expected input).
/// Returns the path to the produced WAV file.
pub async fn convert_to_wav(input: &Path, output_dir: &Path) -> Result<PathBuf> {
    let bin = deps::require(&FFMPEG)?;
    let output = output_dir.join("audio.wav");

    let status = Command::new(bin)
        .arg("-i")
        .arg(input)
        .arg("-ar")
        .arg("16000")
        .arg("-ac")
        .arg("1")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg("-y")
        .arg(&output)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::Audio(format!("spawn ffmpeg: {e}")))?
        .wait_with_output()
        .await
        .map_err(|e| Error::Audio(format!("wait ffmpeg: {e}")))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        let lines: Vec<&str> = stderr.lines().collect();
        let tail = lines[lines.len().saturating_sub(10)..].join("\n");
        return Err(Error::Audio(tail));
    }
    Ok(output)
}

/// Read a 16kHz mono PCM s16le WAV into f32 samples in [-1.0, 1.0].
pub fn read_wav_samples(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| Error::Audio(format!("open wav: {e}")))?;
    let spec = reader.spec();
    if spec.sample_rate != 16_000 || spec.channels != 1 || spec.bits_per_sample != 16 {
        return Err(Error::Audio(format!(
            "unexpected wav format: rate={}, channels={}, bits={}",
            spec.sample_rate, spec.channels, spec.bits_per_sample
        )));
    }
    let samples: std::result::Result<Vec<i16>, _> = reader.samples::<i16>().collect();
    let samples = samples.map_err(|e| Error::Audio(format!("decode wav: {e}")))?;
    Ok(samples.into_iter().map(|s| s as f32 / 32768.0).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_wav(path: &Path, sample_rate: u32, channels: u16) {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(path, spec).unwrap();
        for i in 0..(sample_rate * channels as u32) {
            w.write_sample((i % 100) as i16).unwrap();
        }
        w.finalize().unwrap();
    }

    #[test]
    fn read_valid_wav_returns_samples() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.wav");
        write_test_wav(&p, 16000, 1);
        let samples = read_wav_samples(&p).unwrap();
        assert_eq!(samples.len(), 16000);
    }

    #[test]
    fn read_wrong_format_errors() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.wav");
        write_test_wav(&p, 44100, 2);
        let err = read_wav_samples(&p).unwrap_err();
        assert!(format!("{err}").contains("unexpected wav format"));
    }

    #[tokio::test]
    async fn convert_to_wav_works_when_ffmpeg_available() {
        if which::which("ffmpeg").is_err() {
            eprintln!("skipping: ffmpeg not installed");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        // Generate a 1-second 440Hz sine wav as input.
        let input = dir.path().join("input.wav");
        write_test_wav(&input, 22050, 2);
        let out = convert_to_wav(&input, dir.path()).await.unwrap();
        let samples = read_wav_samples(&out).unwrap();
        // 1 second of 16kHz mono → ~16000 samples.
        assert!((samples.len() as i64 - 16000).abs() < 100);
    }
}
