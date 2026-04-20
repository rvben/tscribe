use crate::transcript::TranscriptEntry;

pub fn render(entry: &TranscriptEntry) -> String {
    serde_json::to_string_pretty(entry).expect("TranscriptEntry serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../tests/fixtures/transcript.json");

    #[test]
    fn renders_canonical_json() {
        let entry: TranscriptEntry = serde_json::from_str(FIXTURE).unwrap();
        let rendered = render(&entry);
        // Round-trip equality
        let reparsed: TranscriptEntry = serde_json::from_str(&rendered).unwrap();
        assert_eq!(entry, reparsed);
        // Pretty-printed (contains a newline + 2-space indent)
        assert!(rendered.contains("\n  "));
    }
}
