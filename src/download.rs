use crate::deps::{self, YT_DLP};
use crate::error::{Error, Result};
use crate::transcript::Metadata;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Deserialize)]
struct YtDlpJson {
    title: Option<String>,
    uploader: Option<String>,
    duration: Option<f64>,
    upload_date: Option<String>, // YYYYMMDD
    webpage_url: Option<String>,
    channel: Option<String>,
    acodec: Option<String>,
    #[serde(default)]
    formats: Vec<YtDlpFormat>,
}

#[derive(Debug, Deserialize)]
struct YtDlpFormat {
    acodec: Option<String>,
}

/// Metadata extracted by [`probe`], used both to drive progress output and to
/// build the final transcript record.
#[derive(Debug, Clone)]
pub struct Probed {
    pub title: Option<String>,
    pub author: Option<String>,
    pub duration_seconds: Option<u64>,
    pub site: Option<String>,
    pub uploaded_at: Option<DateTime<Utc>>,
}

impl Probed {
    /// One-line summary for progress output (e.g. `"Foo" — @bar (3m20s)`).
    pub fn summary(&self) -> String {
        let title = self
            .title
            .as_deref()
            .map(|t| format!("\"{t}\""))
            .unwrap_or_else(|| "(untitled)".to_string());
        let mut out = title;
        if let Some(author) = &self.author {
            out.push_str(" — ");
            out.push_str(author);
        }
        if let Some(secs) = self.duration_seconds {
            out.push_str(&format!(" ({})", format_duration(secs)));
        }
        out
    }

    pub fn into_metadata(self) -> Metadata {
        Metadata {
            title: self.title,
            author: self.author,
            site: self.site,
            duration_seconds: self.duration_seconds,
            uploaded_at: self.uploaded_at,
        }
    }
}

/// Extract metadata via `yt-dlp -J` and verify the media has an audio track.
///
/// This gives a fast, cheap gate before we spend time downloading video-only
/// content (e.g. silent X clips). Returns [`Error::Unsupported`] when there's
/// nothing to transcribe.
pub async fn probe(url: &str) -> Result<Probed> {
    let bin = deps::require(&YT_DLP)?;
    let info = run_yt_dlp_json(&bin, url).await?;

    if !has_audio(&info) {
        return Err(Error::Unsupported(
            "media has no audio track — nothing to transcribe".to_string(),
        ));
    }

    let site = site_from_url(info.webpage_url.as_deref().unwrap_or(url));
    Ok(Probed {
        title: info.title,
        author: info.uploader.or(info.channel),
        duration_seconds: info.duration.map(|d| d as u64),
        site,
        uploaded_at: parse_upload_date(info.upload_date.as_deref()),
    })
}

/// Download the best audio stream to `workdir/audio.mp3`. Expects [`probe`] to
/// have already validated the URL.
pub async fn fetch(url: &str, workdir: &Path) -> Result<PathBuf> {
    let bin = deps::require(&YT_DLP)?;
    let template = workdir.join("audio.%(ext)s");

    let output = Command::new(&bin)
        .arg("--no-playlist")
        .arg("-f")
        .arg("bestaudio/best")
        .arg("--extract-audio")
        .arg("--audio-format")
        .arg("mp3")
        .arg("-o")
        .arg(&template)
        .arg(url)
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::Download(format!("spawn yt-dlp: {e}")))?
        .wait_with_output()
        .await
        .map_err(|e| Error::Download(format!("wait yt-dlp: {e}")))?;

    if !output.status.success() {
        return Err(Error::Download(stderr_tail(&output.stderr)));
    }

    let audio_path = workdir.join("audio.mp3");
    if !audio_path.exists() {
        return Err(Error::Download(format!(
            "yt-dlp finished but {} is missing",
            audio_path.display()
        )));
    }
    Ok(audio_path)
}

async fn run_yt_dlp_json(bin: &Path, url: &str) -> Result<YtDlpJson> {
    let output = Command::new(bin)
        .arg("-J")
        .arg("--no-playlist")
        .arg(url)
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::Download(format!("spawn yt-dlp: {e}")))?
        .wait_with_output()
        .await
        .map_err(|e| Error::Download(format!("wait yt-dlp: {e}")))?;

    if !output.status.success() {
        return Err(Error::Download(stderr_tail(&output.stderr)));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|e| Error::Download(format!("parse yt-dlp json: {e}")))
}

fn stderr_tail(stderr: &[u8]) -> String {
    let s = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = s.lines().collect();
    lines[lines.len().saturating_sub(10)..].join("\n")
}

fn has_audio(info: &YtDlpJson) -> bool {
    // yt-dlp marks video-only formats with acodec == "none" and leaves the
    // field null when it doesn't know. Any real codec name — top-level or in
    // any single format — proves audio is present.
    fn is_real(codec: Option<&str>) -> bool {
        matches!(codec, Some(c) if !c.is_empty() && c != "none")
    }
    is_real(info.acodec.as_deref()) || info.formats.iter().any(|f| is_real(f.acodec.as_deref()))
}

fn site_from_url(url: &str) -> Option<String> {
    url::Url::parse(url).ok().and_then(|u| {
        u.host_str()
            .map(|h| h.trim_start_matches("www.").to_string())
    })
}

fn parse_upload_date(s: Option<&str>) -> Option<DateTime<Utc>> {
    let s = s?;
    if s.len() != 8 {
        return None;
    }
    let y: i32 = s[..4].parse().ok()?;
    let m: u32 = s[4..6].parse().ok()?;
    let d: u32 = s[6..8].parse().ok()?;
    let date = NaiveDate::from_ymd_opt(y, m, d)?;
    Some(Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0)?))
}

fn format_duration(secs: u64) -> String {
    let m = secs / 60;
    let s = secs % 60;
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
    fn parses_upload_date() {
        let dt = parse_upload_date(Some("20260101")).unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-01-01");
    }

    #[test]
    fn rejects_bad_upload_date() {
        assert!(parse_upload_date(Some("nope")).is_none());
        assert!(parse_upload_date(None).is_none());
    }

    #[test]
    fn extracts_site_from_url() {
        assert_eq!(
            site_from_url("https://www.youtube.com/watch?v=x"),
            Some("youtube.com".into())
        );
        assert_eq!(
            site_from_url("https://x.com/u/status/1"),
            Some("x.com".into())
        );
        assert_eq!(site_from_url("not a url"), None);
    }

    fn info(acodec: Option<&str>, formats: &[Option<&str>]) -> YtDlpJson {
        YtDlpJson {
            title: None,
            uploader: None,
            duration: None,
            upload_date: None,
            webpage_url: None,
            channel: None,
            acodec: acodec.map(str::to_owned),
            formats: formats
                .iter()
                .map(|c| YtDlpFormat {
                    acodec: c.map(str::to_owned),
                })
                .collect(),
        }
    }

    #[test]
    fn detects_missing_audio_track() {
        let silent = info(None, &[Some("none"), None, Some("none"), None]);
        assert!(!has_audio(&silent));
    }

    #[test]
    fn detects_audio_from_any_format() {
        let yt = info(Some("opus"), &[Some("none"), Some("mp4a.40.5")]);
        assert!(has_audio(&yt));
    }

    #[test]
    fn format_level_audio_is_enough() {
        let combined = info(None, &[Some("none"), Some("mp4a.40.2")]);
        assert!(has_audio(&combined));
    }

    #[test]
    fn empty_acodec_is_not_audio() {
        let empty = info(Some(""), &[Some("")]);
        assert!(!has_audio(&empty));
    }

    #[test]
    fn summary_renders_all_parts() {
        let p = Probed {
            title: Some("Hello world".into()),
            author: Some("@dimillian".into()),
            duration_seconds: Some(125),
            site: Some("x.com".into()),
            uploaded_at: None,
        };
        assert_eq!(p.summary(), "\"Hello world\" — @dimillian (2m05s)");
    }

    #[test]
    fn summary_handles_missing_parts() {
        let p = Probed {
            title: None,
            author: None,
            duration_seconds: Some(45),
            site: None,
            uploaded_at: None,
        };
        assert_eq!(p.summary(), "(untitled) (45s)");
    }
}
