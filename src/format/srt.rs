use crate::transcript::TranscriptEntry;

pub fn render(entry: &TranscriptEntry) -> String {
    let mut out = String::new();
    for (i, seg) in entry.transcription.segments.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            timestamp(seg.start),
            timestamp(seg.end),
            seg.text.trim()
        ));
    }
    out
}

fn timestamp(seconds: f64) -> String {
    let total_ms = (seconds * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_secs = total_ms / 1000;
    let s = total_secs % 60;
    let m = (total_secs / 60) % 60;
    let h = total_secs / 3600;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/transcript.json");
    const EXPECTED: &str = include_str!("../../tests/fixtures/expected.srt");

    #[test]
    fn renders_srt() {
        let entry: crate::transcript::TranscriptEntry =
            serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(render(&entry), EXPECTED);
    }
}
