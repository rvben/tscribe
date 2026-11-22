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
    validate_threads(opts.threads)?;
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

    let mut out = Vec::with_capacity(state.full_n_segments() as usize);
    for segment in state.as_iter() {
        let text = segment
            .to_str()
            .map_err(|e| Error::Transcribe(format!("get text: {e}")))?
            .to_owned();
        out.push(Segment {
            start: segment.start_timestamp() as f64 / 100.0,
            end: segment.end_timestamp() as f64 / 100.0,
            text,
        });
    }
    Ok(out)
}

pub(crate) fn validate_threads(threads: i32) -> Result<()> {
    if threads < 1 {
        return Err(Error::BadArg(
            "Whisper thread count must be positive".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_thread_counts_are_rejected_before_loading_model() {
        for threads in [0, -1, i32::MIN] {
            let error = transcribe(
                &[],
                TranscribeOptions {
                    model_path: Path::new("unused-model.bin"),
                    language: "en",
                    threads,
                },
            )
            .unwrap_err();
            assert!(matches!(error, Error::BadArg(message) if message.contains("thread count")));
        }
    }
}
