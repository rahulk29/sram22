.PHONY: lint lint-fix format test alltest devtest check install-all install

lint:
	cargo clippy --all-features --all-targets -- -D warnings

lint-fix:
	cargo clippy --fix --allow-staged --allow-dirty --all-features --all-targets
	cargo +nightly fmt

format:
	cargo +nightly fmt
	black scripts/
	black sramgen/scripts/

test:
	cargo test --release

alltest:
	cargo test --release --all-features

check:
	cargo check --all-features --all-targets

install-all:
	cd sramgen && cargo install --all-features --path . && cd -

install:
	cd sramgen && cargo install --path . && cd -
