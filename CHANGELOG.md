# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [0.1.0] - 2026-04-21

### Added

- wire up main entry with subcommand dispatch and exit codes ([5700fa2](https://github.com/rvben/tscribe/commit/5700fa286a759855e9bec75e77bfbd5619878ff7))
- add clap CLI with cache/models/doctor subcommands ([a97f454](https://github.com/rvben/tscribe/commit/a97f45457ecfe77a7a9fd7718c86a0967f84fc19))
- add pipeline orchestrator with cache lookup and tmpdir cleanup ([c444f10](https://github.com/rvben/tscribe/commit/c444f101a4e8a04f2a9ffc34903cebc1a91a2cb5))
- add TTY-aware progress reporter with spinners ([7b2b8c1](https://github.com/rvben/tscribe/commit/7b2b8c1af5231a61a7037fc5b4ffbab2db4d7b62))
- add whisper-rs transcription wrapper ([4028f36](https://github.com/rvben/tscribe/commit/4028f3681d17799d4b749e0b21123fffb6307b5d))
- add yt-dlp wrapper with metadata extraction ([931534e](https://github.com/rvben/tscribe/commit/931534eccfcc29bb26dd87cc26a95e0139fbdca1))
- add ffmpeg-based audio conversion to 16kHz mono PCM ([13fd26b](https://github.com/rvben/tscribe/commit/13fd26bbd39e4ed0064fc19f0fee256bf279ab6c))
- add async model download with progress and atomic write ([19cc43e](https://github.com/rvben/tscribe/commit/19cc43e2b70be1eb6b472de2fdfeb80d2464c61a))
- add model registry with SHA256 verification ([3749782](https://github.com/rvben/tscribe/commit/374978261b6a42babaf737ff1529c0dcc0bc1ff5))
- add system dependency detection with platform-aware install hints ([241168f](https://github.com/rvben/tscribe/commit/241168fdea3cdfb32ab040bd1a73e298c0746c42))
- add transcript cache with sharded layout and schema versioning ([35cbd82](https://github.com/rvben/tscribe/commit/35cbd82f5fa67d53d822c68ccf9f4a0c528284c1))
- add config module with cache paths and model defaults ([1211a0a](https://github.com/rvben/tscribe/commit/1211a0a8c7643b4985ff06c5bd366ceba9b7bbda))
- add markdown formatter with YAML frontmatter and timestamps option ([6558d3c](https://github.com/rvben/tscribe/commit/6558d3c424a624c49ce086ff039359fc69016e77))
- add txt, srt, vtt formatters with paragraph chunking ([ba5a369](https://github.com/rvben/tscribe/commit/ba5a36995cf09c6bd9cbd25d4a0eaf2a5de3984d))
- add Format enum and JSON renderer ([8c13873](https://github.com/rvben/tscribe/commit/8c1387319951a778612b0a432d8541dbe55a76be))
- add canonical TranscriptEntry schema ([ad396b9](https://github.com/rvben/tscribe/commit/ad396b98a6cee2a3565eb786f081748de29cea27))
- add error types with documented exit codes ([ab10e3e](https://github.com/rvben/tscribe/commit/ab10e3ea9bc4a212e898c2d08ec30ac763d9da83))

### Fixed

- **model**: correct SHA256 hashes (previous values were truncated) ([c2cc850](https://github.com/rvben/tscribe/commit/c2cc850bccd0fd381aeeb84517b3f3a169489f97))
- **model**: replace placeholder SHA256s with real values ([8c5cfdf](https://github.com/rvben/tscribe/commit/8c5cfdffff9f4c2a93ec38e71eac9e52857990a9))

## [Unreleased]
