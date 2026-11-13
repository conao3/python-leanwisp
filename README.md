# Leanwisp

A Lean 4 source formatter with a Rust layout engine. Leanwisp normalizes spacing,
indentation, declaration separation, proof and `do` blocks, `match` alternatives,
and line wrapping. It supports Lean 4.35.0, standard library imports, imported
syntax extensions, and local notation/macro declarations with formatter support.

```sh
# Format a file, writing only after validation succeeds.
nix run github:conao3/rust-leanwisp -- source format-source --file Example.lean --write
# Check without writing (254 means formatting changes are needed).
nix run . -- source check-source --file Example.lean
# An editor filter: source on stdin, formatted source on stdout.
nix run . -- --output text source format-source --source file:///dev/stdin
# Choose the target line width (default 100, supported range 20–300).
nix run . -- source format-source --file Example.lean --line-width 80
```

For example:

```lean
-- Input
def double(n:Nat):Nat:=n+n
-- Output
def double (n : Nat) : Nat :=
  n + n
```

Declarations are separated with a blank line and files end with one newline.
Proof bullets, structure fields, `do` statements, and match arms receive canonical
indentation. Groups wrap according to `--line-width`; unbreakable literals and
comments may exceed it. Literal contents, nested comments and their ordering are
preserved. Formatting checks both syntax-tree equality and comment contents,
then checks that a second formatting pass produces the same result. A failure
returns a diagnostic without writing the input.

Lake projects are discovered upward from `--file` (or the current directory for
stdin). `--project-dir PATH` selects one explicitly. Build its imported modules
first with `lake build`; Leanwisp runs its syntax adapter in `lake env` so local
and dependency modules, including Mathlib when available in that project, use
the project's search path. The project's Lean version must match 4.35.0.

The Rust implementation owns command parsing, project discovery, document
rendering, width/indentation policies, validation and file handling. A bundled,
natively compiled Lean adapter uses the real parser and formatter extensions to
produce syntax fingerprints, comment trivia and a formatting document. Imported
modules are loaded; commands that change the parser or scope are elaborated so
later commands can use their syntax. Ordinary declarations and `#eval` commands
are not elaborated. This adapter is intentionally retained rather than attempting
to reimplement Lean's extensible grammar in Rust.

Limitations: syntax extensions need a corresponding Lean formatter; arbitrary
custom elaborators that change grammar indirectly are not automatically handled.
A syntax tree comparison does not prove elaborated theorem equivalence for
macros that inspect source positions. Invalid input, missing modules, unsupported
formatters and unstable layouts are reported as errors. Line endings outside
literals/comments are normalized to LF. Windows is not supported; Linux is tested,
and the Nix flake also declares aarch64-darwin.

CLI inputs are named parameters. `--source` accepts literal text or `file://`
contents; `--file` reads a UTF-8 file. JSON output is the default (`Changed`,
`Source`); global `--output text` emits source and `--output table` reports change
status. Use `leanwisp help`, `leanwisp source help`, and subcommand `--help`.
Exit codes: 0 success, 252 CLI syntax errors, 254 input/formatting/check errors.
Only `format-source --file ... --write` writes, and concurrent changes detected
before the write are rejected. Symlink inputs are rejected.

Development:

```sh
nix develop --command make check  # rustfmt/nixfmt, clippy, all tests
nix build                       # packaged application and integration tests
nix run . -- --version
```

Enable the optional pre-commit hook with `git config core.hooksPath .githooks`.
Tests include golden layout expectations, narrow/wide documents, declarations,
proofs, `do`/`match`/structures, literal/comment preservation, 150 declarations,
Lake-local notation, CLI write/check behavior and compilation of formatted Lean.

Apache-2.0. Leanwisp uses the Lean parser/formatter APIs pinned by `flake.lock`.
The prior Python prototype is retained in Git history; current implementation and
packaging use Rust and the Lean adapter.
