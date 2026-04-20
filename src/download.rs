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
}

pub struct DownloadResult {
    pub audio_path: PathBuf,
    pub metadata: Metadata,
}

/// Download audio via yt-dlp into the given workdir, returning the audio path
/// and parsed metadata.
pub async fn download(url: &str, workdir: &Path) -> Result<DownloadResult> {
    let bin = deps::require(&YT_DLP)?;
    let template = workdir.join("audio.%(ext)s");

    let output = Command::new(&bin)
        .arg("--no-playlist")
        .arg("-f")
        .arg("bestaudio/best")
        .arg("--extract-audio")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--print-json")
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<&str> = stderr.lines().collect();
        let tail = lines[lines.len().saturating_sub(10)..].join("\n");
        return Err(Error::Download(tail));
    }

    // The last line of stdout is the JSON blob (--print-json).
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| l.trim_start().starts_with('{'))
        .ok_or_else(|| Error::Download("yt-dlp produced no JSON metadata".to_string()))?;
    let info: YtDlpJson = serde_json::from_str(json_line)
        .map_err(|e| Error::Download(format!("parse yt-dlp json: {e}")))?;

    let audio_path = workdir.join("audio.mp3");
    if !audio_path.exists() {
        return Err(Error::Download(format!(
            "yt-dlp finished but {} is missing",
            audio_path.display()
        )));
    }

    let metadata = Metadata {
        title: info.title,
        author: info.uploader.or(info.channel),
        site: site_from_url(&info.webpage_url.unwrap_or_else(|| url.to_string())),
        duration_seconds: info.duration.map(|d| d as u64),
        uploaded_at: parse_upload_date(info.upload_date.as_deref()),
    };

    Ok(DownloadResult {
        audio_path,
        metadata,
    })
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
}
