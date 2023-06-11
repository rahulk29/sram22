.PHONY: lint lint-fix format test alltest check install-all install

lint:
	cargo clippy --all-features --all-targets --locked -- -D warnings

lint-fix:
	cargo clippy --fix --allow-staged --allow-dirty --all-features --all-targets --locked
	cargo +nightly fmt

format:
	cargo +nightly fmt

test:
	cargo test --release --locked

alltest:
	cargo test --release --all-features --locked

check:
	cargo check --all-features --all-targets --locked

install-all:
	cargo install --all-features --locked --path .

install:
	cargo install --locked --path .
