.PHONY: check
check:
	nix fmt -- --ci
	cargo clippy --all-targets -- -D warnings
	cargo test

.PHONY: format
format:
	nix fmt
