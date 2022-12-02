.PHONY: lint lint-fix format test alltest devtest check

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
	rm -rf sramgen/build/
	cargo test --release

alltest:
	rm -rf sramgen/build/
	cargo test --release --all-features

devtest:
	rm -rf sramgen/build/
	cargo test --release

check:
	cargo check --all-features --all-targets

