.PHONY: lint lint-fix format test check

lint:
	cargo clippy --all-features --all-targets -- -D warnings

lint-fix:
	cargo clippy --fix --allow-staged --allow-dirty --all-features --all-targets


format:
	cargo fmt

test:
	cargo test

check:
	cargo check --all-features --all-targets
