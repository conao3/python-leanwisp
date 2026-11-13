# Leanwisp

Python CLI with a Lean 4.35.0 parser helper. Follow the shared
notes/new-project-stack.md, notes/lang/python.md and notes/lang/lean.md conventions.
Use uv, hatchling, pytest, ruff, flake-parts and treefmt-nix.
Run `nix develop --command make check` before committing, and `nix build`.
Never replace parser validation with lexical heuristics. Preserve comments,
strings, line endings and indentation; unsupported inputs must fail without writes.
