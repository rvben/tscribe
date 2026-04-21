use crate::audio;
use crate::cache::Cache;
use crate::config::{Paths, default_model_for_lang};
use crate::download;
use crate::error::Result;
use crate::model;
use crate::progress::Reporter;
use crate::transcribe::{self, TranscribeOptions};
use crate::transcript::{Metadata, SCHEMA_VERSION, Segment, TranscriptEntry, Transcription};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct PipelineOptions {
    pub url: String,
    pub language: String,
    pub model_name: Option<String>,
    pub threads: i32,
    pub use_cache: bool,
    pub refresh: bool,
    pub allow_model_download: bool,
}

pub struct PipelineResult {
    pub entry: TranscriptEntry,
    pub from_cache: bool,
}

pub async fn run(
    opts: PipelineOptions,
    paths: &Paths,
    reporter: &Reporter,
) -> Result<PipelineResult> {
    let model_name = opts
        .model_name
        .clone()
        .unwrap_or_else(|| default_model_for_lang(&opts.language).to_string());

    let cache = Cache::new(paths.clone())?;
    let key = Cache::key(&opts.url, &model_name, &opts.language);

    if opts.use_cache
        && !opts.refresh
        && let Some(entry) = cache.get(&key)?
    {
        return Ok(PipelineResult {
            entry,
            from_cache: true,
        });
    }

    let model = model::lookup(&model_name)
        .ok_or_else(|| crate::error::Error::Other(format!("unknown model: {model_name}")))?;

    let pb = reporter.download_bar(None);
    let model_path = if opts.allow_model_download {
        let pb_ref = pb.as_ref();
        model::ensure(model, &paths.model_dir, |dl, total| {
            if let Some(pb) = pb_ref {
                if let Some(t) = total {
                    pb.set_length(t);
                }
                pb.set_position(dl);
            }
        })
        .await?
    } else {
        let path = model::model_path(&paths.model_dir, model.name);
        if !path.exists() {
            return Err(crate::error::Error::ModelDownload(format!(
                "model {} not present and --no-download set",
                model.name
            )));
        }
        path
    };
    reporter.finish(pb, format!("✓ Model ready: {}", model.name));

    let workdir = tempfile::Builder::new().prefix("tscribe-").tempdir()?;

    let probe_pb = reporter.spinner("Checking media...");
    let probed = download::probe(&opts.url).await?;
    reporter.finish(probe_pb, format!("✓ Media: {}", probed.summary()));

    let dl_pb = reporter.spinner("Downloading audio...");
    let audio_path = download::fetch(&opts.url, workdir.path()).await?;
    reporter.finish(dl_pb, "✓ Audio downloaded".to_string());

    let conv_pb = reporter.spinner("Converting audio...");
    let wav = audio::convert_to_wav(&audio_path, workdir.path()).await?;
    reporter.finish(conv_pb, "✓ Audio converted".to_string());

    let samples = audio::read_wav_samples(&wav)?;

    let tx_pb = reporter.spinner("Transcribing...");
    let segments = tokio::task::spawn_blocking({
        let model_path = model_path.clone();
        let language = opts.language.clone();
        let threads = opts.threads;
        move || {
            transcribe::transcribe(
                &samples,
                TranscribeOptions {
                    model_path: &model_path,
                    language: &language,
                    threads,
                },
            )
        }
    })
    .await
    .map_err(|e| crate::error::Error::Transcribe(format!("join error: {e}")))??;
    reporter.finish(tx_pb, format!("✓ Transcribed {} segments", segments.len()));

    let entry = build_entry(
        opts.url.clone(),
        probed.into_metadata(),
        model_name,
        opts.language,
        segments,
    );

    if opts.use_cache || opts.refresh {
        cache.put(&key, &entry)?;
    }

    Ok(PipelineResult {
        entry,
        from_cache: false,
    })
}

fn build_entry(
    url: String,
    metadata: Metadata,
    model: String,
    language: String,
    segments: Vec<Segment>,
) -> TranscriptEntry {
    TranscriptEntry {
        version: SCHEMA_VERSION,
        url,
        metadata,
        transcription: Transcription {
            model,
            language,
            transcribed_at: Utc::now(),
            tscribe_version: env!("CARGO_PKG_VERSION").to_string(),
            segments,
        },
    }
}
