use crate::format::RenderOptions;
use crate::format::txt::build_paragraphs;
use crate::transcript::TranscriptEntry;

pub fn render(entry: &TranscriptEntry, opts: RenderOptions) -> String {
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&frontmatter(entry));
    out.push_str("---\n\n");

    if let Some(title) = entry.metadata.title.as_deref() {
        out.push_str(&format!("# {title}\n\n"));
    }

    if opts.timestamps {
        // One line per paragraph with leading [MM:SS] from first segment.
        let mut paragraphs: Vec<(f64, String)> = Vec::new();
        let mut current_start: Option<f64> = None;
        let mut current_segs: Vec<&crate::transcript::Segment> = Vec::new();

        let segs = &entry.transcription.segments;
        for (i, seg) in segs.iter().enumerate() {
            if current_start.is_none() {
                current_start = Some(seg.start);
            }
            current_segs.push(seg);

            let next_pause = segs.get(i + 1).map(|n| n.start - seg.end).unwrap_or(f64::MAX);
            let ends_sentence = matches!(
                seg.text.trim().chars().last(),
                Some('.' | '?' | '!')
            );
            let break_now = next_pause >= 2.0
                || current_segs.len() >= 6
                || (ends_sentence && next_pause >= 1.5);

            if break_now {
                let joined = current_segs
                    .iter()
                    .map(|s| s.text.trim())
                    .collect::<Vec<_>>()
                    .join(" ");
                paragraphs.push((current_start.unwrap(), joined));
                current_start = None;
                current_segs.clear();
            }
        }

        for (i, (start, text)) in paragraphs.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&format!("[{}] {text}\n", mm_ss(*start)));
        }
    } else {
        let paragraphs = build_paragraphs(&entry.transcription.segments);
        out.push_str(&paragraphs.join("\n\n"));
        out.push('\n');
    }

    out
}

fn frontmatter(entry: &TranscriptEntry) -> String {
    let mut out = String::new();
    out.push_str(&format!("source: {}\n", entry.url));
    if let Some(title) = entry.metadata.title.as_deref() {
        out.push_str(&format!("title: {title}\n"));
    }
    if let Some(author) = entry.metadata.author.as_deref() {
        out.push_str(&format!("author: {author}\n"));
    }
    if let Some(site) = entry.metadata.site.as_deref() {
        out.push_str(&format!("site: {site}\n"));
    }
    if let Some(d) = entry.metadata.duration_seconds {
        out.push_str(&format!("duration: \"{}\"\n", hh_mm_ss(d)));
    }
    out.push_str(&format!("language: {}\n", entry.transcription.language));
    out.push_str(&format!("model: {}\n", entry.transcription.model));
    out.push_str(&format!(
        "transcribed_at: {}\n",
        entry
            .transcription
            .transcribed_at
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    ));
    out.push_str(&format!(
        "tscribe_version: {}\n",
        entry.transcription.tscribe_version
    ));
    out
}

fn hh_mm_ss(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds / 60) % 60;
    let s = seconds % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

fn mm_ss(seconds: f64) -> String {
    let total = seconds.round() as u64;
    let m = total / 60;
    let s = total % 60;
    format!("{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/transcript.json");
    const EXPECTED: &str = include_str!("../../tests/fixtures/expected.md");
    const EXPECTED_TS: &str = include_str!("../../tests/fixtures/expected_timestamps.md");

    #[test]
    fn renders_markdown_default() {
        let entry: TranscriptEntry = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(render(&entry, RenderOptions::default()), EXPECTED);
    }

    #[test]
    fn renders_markdown_with_timestamps() {
        let entry: TranscriptEntry = serde_json::from_str(FIXTURE).unwrap();
        let opts = RenderOptions { timestamps: true };
        assert_eq!(render(&entry, opts), EXPECTED_TS);
    }
}
