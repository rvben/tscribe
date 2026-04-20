use crate::transcript::{Segment, TranscriptEntry};

const PARAGRAPH_BREAK_PAUSE_S: f64 = 2.0;
const SAME_SENTENCE_PAUSE_S: f64 = 1.5;
const MAX_SEGMENTS_PER_PARAGRAPH: usize = 6;

pub fn render(entry: &TranscriptEntry) -> String {
    let paragraphs = build_paragraphs(&entry.transcription.segments);
    paragraphs.join("\n\n") + "\n"
}

pub(crate) fn build_paragraphs(segments: &[Segment]) -> Vec<String> {
    if segments.is_empty() {
        return Vec::new();
    }

    let mut paragraphs: Vec<String> = Vec::new();
    let mut current: Vec<&Segment> = Vec::new();

    for seg in segments {
        if let Some(prev) = current.last() {
            let pause = seg.start - prev.end;
            let break_paragraph = pause >= PARAGRAPH_BREAK_PAUSE_S
                || current.len() >= MAX_SEGMENTS_PER_PARAGRAPH
                || (ends_sentence(&prev.text) && pause >= SAME_SENTENCE_PAUSE_S);

            if break_paragraph {
                paragraphs.push(join_segments(&current));
                current.clear();
            }
        }
        current.push(seg);
    }
    if !current.is_empty() {
        paragraphs.push(join_segments(&current));
    }
    paragraphs
}

fn join_segments(segs: &[&Segment]) -> String {
    segs.iter()
        .map(|s| s.text.trim())
        .collect::<Vec<_>>()
        .join(" ")
}

fn ends_sentence(text: &str) -> bool {
    matches!(text.trim().chars().last(), Some('.' | '?' | '!'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::TranscriptEntry;

    const FIXTURE: &str = include_str!("../../tests/fixtures/transcript.json");
    const EXPECTED: &str = include_str!("../../tests/fixtures/expected.txt");

    #[test]
    fn renders_paragraphs() {
        let entry: TranscriptEntry = serde_json::from_str(FIXTURE).unwrap();
        let rendered = render(&entry);
        assert_eq!(rendered, EXPECTED);
    }
}
