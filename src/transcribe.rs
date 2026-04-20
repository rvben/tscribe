use crate::error::{Error, Result};
use crate::transcript::Segment;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct TranscribeOptions<'a> {
    pub model_path: &'a Path,
    pub language: &'a str,
    pub threads: i32,
}

pub fn transcribe(samples: &[f32], opts: TranscribeOptions<'_>) -> Result<Vec<Segment>> {
    let ctx = WhisperContext::new_with_params(
        opts.model_path
            .to_str()
            .ok_or_else(|| Error::Transcribe("model path is not valid UTF-8".into()))?,
        WhisperContextParameters::default(),
    )
    .map_err(|e| Error::Transcribe(format!("load model: {e}")))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(opts.threads);
    params.set_language(Some(opts.language));
    params.set_print_realtime(false);
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);

    let mut state = ctx
        .create_state()
        .map_err(|e| Error::Transcribe(format!("create state: {e}")))?;
    state
        .full(params, samples)
        .map_err(|e| Error::Transcribe(format!("full: {e}")))?;

    let n_segments = state
        .full_n_segments()
        .map_err(|e| Error::Transcribe(format!("count segments: {e}")))?;
    let mut out = Vec::with_capacity(n_segments as usize);
    for i in 0..n_segments {
        let text = state
            .full_get_segment_text(i)
            .map_err(|e| Error::Transcribe(format!("get text: {e}")))?;
        let start = state
            .full_get_segment_t0(i)
            .map_err(|e| Error::Transcribe(format!("get t0: {e}")))? as f64
            / 100.0;
        let end = state
            .full_get_segment_t1(i)
            .map_err(|e| Error::Transcribe(format!("get t1: {e}")))? as f64
            / 100.0;
        out.push(Segment { start, end, text });
    }
    Ok(out)
}
