use clap::{CommandFactory, Parser};
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;
use tokio::runtime::Runtime;
use tscribe::cache::Cache;
use tscribe::cli::{CacheAction, Cli, Command, ModelAction};
use tscribe::config::Paths;
use tscribe::deps::{self, FFMPEG, YT_DLP};
use tscribe::error::Error;
use tscribe::format::{self, RenderOptions};
use tscribe::logging;
use tscribe::model;
use tscribe::pipeline::{self, PipelineOptions};
use tscribe::progress::{Reporter, Verbosity};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let paths = Paths::discover();

    let result = match cli.command {
        Some(Command::Cache { action }) => run_cache(action, &paths),
        Some(Command::Models { action }) => run_models(action, &paths),
        Some(Command::Doctor) => run_doctor(&paths),
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
            eprintln!("error: {e}");
            ExitCode::from(e.exit_code() as u8)
        }
    }
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
            eprintln!("✓ Wrote {}", path.display());
        }
    } else {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(rendered.as_bytes())?;
    }

    if !result.from_cache && verbosity != Verbosity::Quiet {
        let secs = elapsed.as_secs();
        eprintln!("✓ Done ({}m{:02}s)", secs / 60, secs % 60);
    }
    Ok(())
}

fn num_cpus_physical() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn run_cache(action: CacheAction, paths: &Paths) -> Result<(), Error> {
    let cache = Cache::new(paths.clone())?;
    match action {
        CacheAction::List => {
            for (key, entry) in cache.list()? {
                println!(
                    "{}  {}  {}  {}",
                    &key[..12],
                    entry.transcribed_at.format("%Y-%m-%d"),
                    entry.model,
                    entry.url
                );
            }
        }
        CacheAction::Clear => {
            cache.clear()?;
            eprintln!("✓ Cleared cache at {}", paths.transcript_dir.display());
        }
        CacheAction::Path => {
            println!("{}", paths.cache_dir.display());
        }
    }
    Ok(())
}

fn run_models(action: ModelAction, paths: &Paths) -> Result<(), Error> {
    match action {
        ModelAction::List => {
            for m in model::REGISTRY {
                let path = model::model_path(&paths.model_dir, m.name);
                let status = if path.exists() {
                    "✓ installed"
                } else {
                    "  not installed"
                };
                println!("{:10}  {:>5} MB  {}", m.name, m.size_mb, status);
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
            reporter.finish(pb, format!("✓ Downloaded {}", m.name));
        }
        ModelAction::Clear => {
            if paths.model_dir.exists() {
                std::fs::remove_dir_all(&paths.model_dir)?;
            }
            eprintln!("✓ Cleared models at {}", paths.model_dir.display());
        }
    }
    Ok(())
}

fn run_doctor(paths: &Paths) -> Result<(), Error> {
    println!("tscribe v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("System dependencies:");
    for dep in [&YT_DLP, &FFMPEG] {
        match deps::locate(dep) {
            Some(path) => println!("  ✓ {:8} {}", dep.name, path.display()),
            None => println!(
                "  ✗ {:8} (missing)\n      install: {}",
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
    for m in model::REGISTRY {
        let p = model::model_path(&paths.model_dir, m.name);
        if p.exists() {
            any = true;
            println!("  ✓ {:10} {:>5} MB", m.name, m.size_mb);
        }
    }
    if !any {
        println!("  (none yet — will download on first use)");
    }
    Ok(())
}
