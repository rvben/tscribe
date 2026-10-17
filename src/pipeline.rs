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
        reporter.finish(None, format!("⚡ Cache hit: {}", entry.metadata.summary()));
        return Ok(PipelineResult {
            entry,
            from_cache: true,
        });
    }

    let model = model::lookup(&model_name)
        .ok_or_else(|| crate::error::Error::BadArg(model::unknown_model_message(&model_name)))?;

    // Probe the URL first so a bad link fails before we spend time fetching a
    // multi-hundred-MB model.
    let probe_pb = reporter.spinner("Checking media...");
    let probed = match download::probe(&opts.url).await {
        Ok(p) => {
            reporter.finish(probe_pb, format!("✓ Media: {}", p.summary()));
            p
        }
        Err(e) => {
            reporter.fail(probe_pb, "✗ Checking media...".to_string());
            return Err(e);
        }
    };

    let workdir = tempfile::Builder::new().prefix("tscribe-").tempdir()?;
    let (model_path, wav) = prepare_inputs(&opts, paths, reporter, model, workdir.path()).await?;

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
    .map_err(|e| crate::error::Error::Transcribe(format!("join error: {e}")));
    let segments = match segments.and_then(|r| r) {
        Ok(s) => {
            reporter.finish(tx_pb, format!("✓ Transcribed {} segments", s.len()));
            s
        }
        Err(e) => {
            reporter.fail(tx_pb, "✗ Transcribing...".to_string());
            return Err(e);
        }
    };

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

// The media probe and cache lookup have already succeeded. Both preparation
// branches are independent until transcription needs the model and WAV together.
async fn prepare_inputs(
    opts: &PipelineOptions,
    paths: &Paths,
    reporter: &Reporter,
    model: &model::Model,
    workdir: &std::path::Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
    if !opts.allow_model_download && !model::model_path(&paths.model_dir, model.name).exists() {
        return Err(crate::error::Error::ModelMissing(format!(
            "model {} not installed (run `tscribe models download {}` or drop --no-download)",
            model.name, model.name
        )));
    }
    let prepare_model = async {
        let pb = reporter.download_bar(None);
        let result = if opts.allow_model_download {
            let pb_ref = pb.as_ref();
            model::ensure(model, &paths.model_dir, |dl, total| {
                if let Some(pb) = pb_ref {
                    if let Some(t) = total {
                        pb.set_length(t);
                    }
                    pb.set_position(dl);
                }
            })
            .await
        } else {
            let path = model::model_path(&paths.model_dir, model.name);
            Ok(path)
        };
        match result {
            Ok(path) => {
                reporter.finish(pb, format!("✓ Model ready: {}", model.name));
                Ok(path)
            }
            Err(error) => {
                reporter.fail(pb, "✗ Preparing model...".to_string());
                Err(error)
            }
        }
    };
    let prepare_audio = async {
        let dl_pb = reporter.spinner("Downloading audio...");
        let audio_path = match download::fetch(&opts.url, workdir).await {
            Ok(p) => {
                reporter.finish(dl_pb, "✓ Audio downloaded".to_string());
                p
            }
            Err(e) => {
                reporter.fail(dl_pb, "✗ Downloading audio...".to_string());
                return Err(e);
            }
        };

        let conv_pb = reporter.spinner("Converting audio...");
        let wav = match audio::convert_to_wav(&audio_path, workdir).await {
            Ok(w) => {
                reporter.finish(conv_pb, "✓ Audio converted".to_string());
                w
            }
            Err(e) => {
                reporter.fail(conv_pb, "✗ Converting audio...".to_string());
                return Err(e);
            }
        };

        Ok(wav)
    };
    // Dropping the other branch on failure cleans up partial model files and
    // kills its media child process before the temporary work directory goes.
    tokio::try_join!(prepare_model, prepare_audio)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::Verbosity;
    use std::{io::Cursor, sync::Arc, time::Duration};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        sync::Notify,
    };
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

    fn options(url: String) -> PipelineOptions {
        PipelineOptions {
            url,
            language: "en".into(),
            model_name: None,
            threads: 1,
            use_cache: false,
            refresh: false,
            allow_model_download: true,
        }
    }

    #[tokio::test]
    async fn audio_download_proceeds_while_model_download_is_pending() {
        if which::which("yt-dlp").is_err() || which::which("ffmpeg").is_err() {
            eprintln!("skipping: yt-dlp and ffmpeg required");
            return;
        }
        let media = MockServer::start().await;
        let audio_requested = Arc::new(Notify::new());
        let signal = audio_requested.clone();
        let mut wav = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(
            &mut wav,
            hound::WavSpec {
                channels: 1,
                sample_rate: 16_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..16_000 {
            writer.write_sample(0_i16).unwrap();
        }
        writer.finalize().unwrap();
        let audio = wav.into_inner();
        Mock::given(path("/audio.wav"))
            .respond_with(move |_: &wiremock::Request| {
                signal.notify_one();
                ResponseTemplate::new(200)
                    .set_body_bytes(audio.clone())
                    .insert_header("content-type", "audio/wav")
            })
            .mount(&media)
            .await;

        // The model server will not finish until yt-dlp requests the audio.
        // A sequential pipeline deadlocks here and fails the timeout.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url =
            Box::leak(format!("http://{}/model", listener.local_addr().unwrap()).into_boxed_str());
        let model_server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            assert!(stream.read(&mut request).await.unwrap() > 0);
            audio_requested.notified().await;
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
                )
                .await
                .unwrap();
        });
        let model = model::Model {
            name: "test",
            size_mb: 0,
            url,
            sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
            multilingual: false,
        };
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::with_root(dir.path().join("cache"));
        let result = tokio::time::timeout(
            Duration::from_secs(20),
            prepare_inputs(
                &options(format!("{}/audio.wav", media.uri())),
                &paths,
                &Reporter::new(Verbosity::Quiet),
                &model,
                dir.path(),
            ),
        )
        .await;
        model_server.abort();
        let (model_path, wav) = result
            .expect("model and media preparation must overlap")
            .unwrap();
        assert_eq!(std::fs::read(model_path).unwrap(), b"hello");
        assert!(!audio::read_wav_samples(&wav).unwrap().is_empty());
    }

    #[tokio::test]
    async fn missing_model_with_download_disabled_does_not_fetch_media() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let mut opts = options(format!("{}/audio.wav", server.uri()));
        opts.allow_model_download = false;
        let error = prepare_inputs(
            &opts,
            &Paths::with_root(dir.path().into()),
            &Reporter::new(Verbosity::Quiet),
            model::lookup("tiny.en").unwrap(),
            dir.path(),
        )
        .await
        .unwrap_err();
        assert!(matches!(error, crate::error::Error::ModelMissing(_)));
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}
