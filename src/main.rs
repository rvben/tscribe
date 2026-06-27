use clap::{CommandFactory, Parser};
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;
use tokio::runtime::Runtime;
use tscribe::cache::Cache;
use tscribe::cli::{CacheAction, Cli, Command, ModelAction, ResolvedOutputMode};
use tscribe::config::Paths;
use tscribe::deps::{self, FFMPEG, YT_DLP};
use tscribe::error::Error;
use tscribe::format::{self, RenderOptions};
use tscribe::logging;
use tscribe::model;
use tscribe::pipeline::{self, PipelineOptions};
use tscribe::progress::{Reporter, Verbosity};
use tscribe::schema;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let paths = Paths::discover();
    // --json flag is a shorthand for --output-mode json.
    let output_mode = cli.output_mode.with_json_flag(cli.json).resolve();

    let result = match cli.command {
        Some(Command::Cache { action }) => run_cache(action, &paths, output_mode),
        Some(Command::Models { action }) => run_models(action, &paths, output_mode),
        Some(Command::Doctor {
            limit,
            offset,
            fields,
            format,
        }) => {
            // --format on the doctor subcommand overrides the global --output-mode.
            let effective_mode = format.map(|m| m.resolve()).unwrap_or(output_mode);
            run_doctor(&paths, effective_mode, limit, offset, fields)
        }
        Some(Command::Schema) => run_schema(),
        Some(Command::Completions { shell }) => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "tscribe", &mut std::io::stdout());
            Ok(())
        }
        None => match cli.url.clone() {
            Some(url) => run_transcribe(url, cli, &paths),
            None => {
                Cli::command().print_help().ok();
                println!();
                Ok(())
            }
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            e.emit_structured();
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

fn run_schema() -> Result<(), Error> {
    let s = schema::build();
    let json = serde_json::to_string_pretty(&s)?;
    println!("{json}");
    Ok(())
}

fn run_transcribe(url: String, cli: Cli, paths: &Paths) -> Result<(), Error> {
    let args = cli.transcribe;
    let verbosity = Verbosity::from_flags(args.quiet, args.verbose);
    logging::init(args.verbose);
    let reporter = Reporter::new(verbosity);

    let threads = args
        .threads
        .map(|n| n as i32)
        .unwrap_or_else(|| num_cpus_physical() as i32);

    let opts = PipelineOptions {
        url,
        language: args.lang.clone(),
        model_name: args.model.clone(),
        threads,
        use_cache: !args.no_cache,
        refresh: args.refresh,
        allow_model_download: !args.no_download,
    };

    let started = Instant::now();
    let rt = Runtime::new().map_err(|e| Error::Other(format!("create runtime: {e}")))?;
    let result = rt.block_on(pipeline::run(opts, paths, &reporter))?;
    let elapsed = started.elapsed();

    let rendered = format::render(
        &result.entry,
        args.format,
        RenderOptions {
            timestamps: args.timestamps,
        },
    );

    if let Some(path) = args.output {
        std::fs::write(&path, &rendered)?;
        if verbosity != Verbosity::Quiet {
            eprintln!("Wrote {}", path.display());
        }
    } else {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(rendered.as_bytes())?;
    }

    if !result.from_cache && verbosity != Verbosity::Quiet {
        let secs = elapsed.as_secs();
        eprintln!("Done ({}m{:02}s)", secs / 60, secs % 60);
    }
    Ok(())
}

fn num_cpus_physical() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn run_cache(
    action: CacheAction,
    paths: &Paths,
    output_mode: ResolvedOutputMode,
) -> Result<(), Error> {
    let cache = Cache::new(paths.clone())?;
    match action {
        CacheAction::List {
            limit,
            offset,
            fields,
            format,
        } => {
            // A per-command --format overrides the global --output-mode.
            let output_mode = format.map(|m| m.resolve()).unwrap_or(output_mode);
            let all_rows = cache.list()?;
            let total = all_rows.len();
            let rows: Vec<_> = all_rows.into_iter().skip(offset).take(limit).collect();

            let active_fields: Option<Vec<&str>> = fields
                .as_deref()
                .map(|f| f.split(',').map(str::trim).collect());

            if output_mode == ResolvedOutputMode::Json {
                let items: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|(key, entry)| {
                        let mut obj = serde_json::json!({
                            "key": &key[..12.min(key.len())],
                            "url": entry.url,
                            "model": entry.model,
                            "language": entry.language,
                            "date": entry.transcribed_at.format("%Y-%m-%d").to_string(),
                            "title": entry.title
                        });
                        if let Some(ref af) = active_fields {
                            let map = obj.as_object_mut().unwrap();
                            map.retain(|k, _| af.contains(&k.as_str()));
                        }
                        obj
                    })
                    .collect();
                let envelope = serde_json::json!({
                    "items": items,
                    "total": total,
                    "limit": limit,
                    "offset": offset
                });
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else if rows.is_empty() {
                eprintln!("(cache empty)");
            } else {
                let model_w = rows
                    .iter()
                    .map(|(_, e)| e.model.len())
                    .max()
                    .unwrap_or(5)
                    .max(5);
                println!(
                    "{:<12}  {:<10}  {:<width$}  URL",
                    "KEY",
                    "DATE",
                    "MODEL",
                    width = model_w
                );
                for (key, entry) in &rows {
                    println!(
                        "{:<12}  {:<10}  {:<width$}  {}",
                        &key[..12.min(key.len())],
                        entry.transcribed_at.format("%Y-%m-%d"),
                        entry.model,
                        entry.url,
                        width = model_w
                    );
                }
            }
        }
        CacheAction::Clear => {
            cache.clear()?;
            eprintln!("Cleared cache at {}", paths.transcript_dir.display());
        }
        CacheAction::Path => {
            println!("{}", paths.cache_dir.display());
        }
    }
    Ok(())
}

fn run_models(
    action: ModelAction,
    paths: &Paths,
    output_mode: ResolvedOutputMode,
) -> Result<(), Error> {
    match action {
        ModelAction::List { format } => {
            // A per-command --format overrides the global --output-mode.
            let output_mode = format.map(|m| m.resolve()).unwrap_or(output_mode);
            if output_mode == ResolvedOutputMode::Json {
                let items: Vec<serde_json::Value> = model::REGISTRY
                    .iter()
                    .map(|m| {
                        let path = model::model_path(&paths.model_dir, m.name);
                        serde_json::json!({
                            "name": m.name,
                            "size_mb": m.size_mb,
                            "installed": path.exists(),
                            "multilingual": m.multilingual
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                for m in model::REGISTRY {
                    let path = model::model_path(&paths.model_dir, m.name);
                    let status = if path.exists() {
                        "installed"
                    } else {
                        "not installed"
                    };
                    println!("{:10}  {:>5} MB  {}", m.name, m.size_mb, status);
                }
            }
        }
        ModelAction::Download { name } => {
            let m = model::lookup(&name)
                .ok_or_else(|| Error::BadArg(model::unknown_model_message(&name)))?;
            let reporter = Reporter::new(Verbosity::Normal);
            let pb = reporter.download_bar(None);
            let pb_ref = pb.as_ref();
            let rt = Runtime::new().map_err(|e| Error::Other(format!("create runtime: {e}")))?;
            rt.block_on(model::ensure(m, &paths.model_dir, |dl, total| {
                if let Some(pb) = pb_ref {
                    if let Some(t) = total {
                        pb.set_length(t);
                    }
                    pb.set_position(dl);
                }
            }))?;
            reporter.finish(pb, format!("Downloaded {}", m.name));
        }
        ModelAction::Clear => {
            if paths.model_dir.exists() {
                std::fs::remove_dir_all(&paths.model_dir)?;
            }
            eprintln!("Cleared models at {}", paths.model_dir.display());
        }
    }
    Ok(())
}

fn run_doctor(
    paths: &Paths,
    output_mode: ResolvedOutputMode,
    limit: usize,
    offset: usize,
    fields: Option<String>,
) -> Result<(), Error> {
    if output_mode == ResolvedOutputMode::Json {
        let deps_info: Vec<serde_json::Value> = [&YT_DLP, &FFMPEG]
            .iter()
            .map(|dep| {
                let found = deps::locate(dep).map(|p| p.display().to_string());
                serde_json::json!({
                    "name": dep.name,
                    "found": found.is_some(),
                    "path": found,
                    "install_hint": dep.install_hint
                })
            })
            .collect();

        let all_models: Vec<serde_json::Value> = model::REGISTRY
            .iter()
            .filter_map(|m| {
                let p = model::model_path(&paths.model_dir, m.name);
                if p.exists() {
                    Some(serde_json::json!({
                        "name": m.name,
                        "size_mb": m.size_mb,
                        "path": p.display().to_string()
                    }))
                } else {
                    None
                }
            })
            .collect();

        let total_models = all_models.len();
        let shown_models: Vec<_> = all_models.into_iter().skip(offset).take(limit).collect();

        let active_fields: Option<Vec<&str>> = fields
            .as_deref()
            .map(|f| f.split(',').map(str::trim).collect());

        let mut result = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "dependencies": deps_info,
            "models": shown_models,
            "models_total": total_models,
            "cache_dir": paths.cache_dir.display().to_string(),
            "model_dir": paths.model_dir.display().to_string()
        });

        if let Some(ref af) = active_fields {
            let map = result.as_object_mut().unwrap();
            map.retain(|k, _| af.contains(&k.as_str()));
        }

        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("tscribe v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("System dependencies:");
        for dep in [&YT_DLP, &FFMPEG] {
            match deps::locate(dep) {
                Some(path) => println!("  ok {:8} {}", dep.name, path.display()),
                None => println!(
                    "  missing {:8}\n      install: {}",
                    dep.name, dep.install_hint
                ),
            }
        }
        println!();
        println!("Cache:    {}", paths.cache_dir.display());
        println!("Models:   {}", paths.model_dir.display());
        println!();
        println!("Models on disk:");
        let mut any = false;
        for m in model::REGISTRY.iter().skip(offset).take(limit) {
            let p = model::model_path(&paths.model_dir, m.name);
            if p.exists() {
                any = true;
                println!("  ok {:10} {:>5} MB", m.name, m.size_mb);
            }
        }
        if !any {
            println!("  (none yet - will download on first use)");
        }
    }
    Ok(())
}
