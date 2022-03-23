.PHONY: lint lint-fix format test check

lint:
	cargo clippy --all-features --all-targets -- -D warnings

lint-fix:
	cargo clippy --fix --allow-staged --allow-dirty --all-features --all-targets
	cargo fmt


format:
	cargo fmt
	black scripts/

test:
	rm -rf /tmp/sram22/
	cargo test

check:
	cargo check --all-features --all-targets

run:
	rm -rf _build/ && cargo run --release -- configs/sram_16x16.toml

