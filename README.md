# tscribe

[![crates.io](https://img.shields.io/crates/v/tscribe.svg)](https://crates.io/crates/tscribe)
[![CI](https://github.com/rvben/tscribe/actions/workflows/ci.yml/badge.svg)](https://github.com/rvben/tscribe/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> Transcribe any video/audio URL into agent-friendly markdown.

`tscribe` shells out to [yt-dlp](https://github.com/yt-dlp/yt-dlp) for
downloading, [ffmpeg](https://ffmpeg.org/) for audio conversion, and embeds
[whisper.cpp](https://github.com/ggerganov/whisper.cpp) (via
[`whisper-rs`](https://crates.io/crates/whisper-rs)) for transcription.
Transcripts are cached so re-running is instant.

## Install

### macOS / Linux (Homebrew)

```sh
brew install rvben/tap/tscribe
```

This pulls in `yt-dlp` and `ffmpeg` automatically.

### Cargo

```sh
cargo install tscribe
```

You'll need `yt-dlp` and `ffmpeg` separately.

### Prebuilt binaries

Grab the latest from [Releases](https://github.com/rvben/tscribe/releases).

## Quickstart

```sh
# Transcribe an X (Twitter) video
tscribe https://x.com/user/status/123456789

# Save to a file
tscribe https://youtu.be/dQw4w9WgXcQ -o talk.md

# Different language
tscribe --lang nl https://podcast.example.com/episode-12.mp3

# Different format
tscribe --format json https://x.com/.../123 | jq '.transcription.segments[0]'

# Diagnose installation
tscribe doctor
```

## Output

By default, `tscribe` outputs markdown with YAML frontmatter:

```markdown
---
source: https://x.com/...
title: ...
author: "@..."
duration: "00:55:40"
language: en
model: small.en
transcribed_at: 2026-04-20T21:30:00Z
tscribe_version: 0.1.0
---

# Title

Transcript paragraphs here...
```

Other formats: `txt`, `json`, `srt`, `vtt`.

## Models

First run downloads the default model (`small.en`, ~466 MB) to the cache dir.

| Model | Size | Notes |
|---|---|---|
| `tiny.en` | 39 MB | Fastest |
| `base.en` | 142 MB | Good quality |
| `small.en` | 466 MB | **Default** |
| `medium.en` | 1.4 GB | Excellent |
| `small` | 466 MB | Multilingual (auto for `--lang` ≠ `en`) |
| `large-v3` | 2.9 GB | Best, multilingual |

Pre-download: `tscribe models download <name>`.
List: `tscribe models list`.

## Cache

Transcripts are cached at:

- macOS: `~/Library/Caches/tscribe/transcripts/`
- Linux: `~/.cache/tscribe/transcripts/`

Override with `TSCRIBE_CACHE_DIR`. Clear with `tscribe cache clear`.

Re-running on a previously-transcribed URL is instant.

## Subcommands

```
tscribe <URL>             Transcribe (primary mode)
tscribe cache list        Show cached transcripts
tscribe cache clear       Nuke cache
tscribe cache path        Print cache directory
tscribe models list       Show downloaded models
tscribe models download   Pre-download a model
tscribe models clear      Remove all models
tscribe doctor            Check yt-dlp, ffmpeg, models
tscribe completions <shell>  Generate shell completions
```

## Exit codes

```
0   Success
1   Generic error
2   Bad URL / unsupported site
3   Download failed (yt-dlp)
4   Transcription failed
5   Missing system dep
6   Model download failed
130 Interrupted
```

## License

`tscribe` is released under the [MIT License](LICENSE). Third-party dependencies
are covered by their own licenses; a full `THIRD_PARTY_LICENSES.md` manifest is
generated via `cargo-about` and bundled into each release tarball. Regenerate
locally with `make licenses`.
