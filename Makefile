.PHONY: lint lint-fix format test devtest check

lint:
	cargo clippy --all-features --all-targets -- -D warnings

lint-fix:
	cargo clippy --fix --allow-staged --allow-dirty --all-features --all-targets
	cargo fmt


format:
	cargo +nightly fmt
	black scripts/
	black sramgen/scripts/

test:
	rm -rf sramgen/build/
	cargo test --release --features calibre --features spectre

devtest:
	rm -rf sramgen/build/
	cargo test --release

check:
	cargo check --all-features --all-targets

