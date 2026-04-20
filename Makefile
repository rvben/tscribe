.PHONY: init build test lint fmt fmt-check clippy nextest deny ci clean release-patch release-minor release-major

init:
	@which cargo > /dev/null || (echo "cargo not found, install from https://rustup.rs" && exit 1)
	@which cargo-nextest > /dev/null 2>&1 || cargo install cargo-nextest --locked
	@which cargo-deny > /dev/null 2>&1 || cargo install cargo-deny --locked

build:
	cargo build --release

test: nextest

nextest:
	cargo nextest run --no-fail-fast

lint: fmt-check clippy

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

deny:
	cargo deny check

ci: fmt-check clippy nextest deny

clean:
	cargo clean

release-patch:
	vership bump patch

release-minor:
	vership bump minor

release-major:
	vership bump major
