.PHONY: fmt lint test run-cli deny audit

fmt:
	cargo fmt

lint:
	cargo fmt -- --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	RUSTFLAGS='-C overflow-checks=on' cargo test --all-features

run-cli:
	cargo run --bin astreinte-cli --features serde

deny:
	cargo deny check || true

audit:
	cargo audit || true
