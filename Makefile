.PHONY: check
check:
	nix fmt -- --ci
	uv run pytest

.PHONY: format
format:
	nix fmt
