# Leanwisp

A conservative Lean 4 source formatter. Version 0.1.0 targets Lean 4.35.0
and the built-in Init syntax. Python handles the CLI and byte edits; a bundled
Lean helper uses Lean's own parser, without elaborating or executing the input.

```sh
nix run . -- --output text source format-source --source 'file:///dev/stdin'
nix run . -- source format-source --file Example.lean --write
nix run . -- source check-source --file Example.lean
nix develop --command make check
nix build
```

For example, `#eval (3: Nat) + 4` becomes `#eval (3 : Nat) + 4`.
Named inputs only: `--source` accepts literal text or `file://` contents;
`--file` reads a UTF-8 file. Formatting returns JSON (`Changed`, `Source`) by
default. Global `--output text` emits only source, suitable for an editor filter.
Use `leanwisp help`, `leanwisp source help`, or subcommand `help`.
No writes happen without `format-source --file ... --write`.

The formatter normalizes same-line spaces around `:`, `:=`, `=>`, arrows,
`=`, `+`, `*`, and after commas, using parser token boundaries. It preserves
comments, literal contents, line breaks (including CRLF), indentation and EOF.
It reparses the result and rejects changes to node kinds, identifiers or atoms.
This catches layout-sensitive changes despite identical token spellings.
It does not reflow proofs or enforce the entire
[Lean standard library style guide](https://github.com/leanprover/lean4/blob/master/doc/std/style.md).

**Limits:** additional imports (including Std/Mathlib), parser-changing commands
and incomplete source are rejected. No project dependencies are loaded. This is
an intentionally restricted first release, not a general Mathlib formatter.
Syntax-tree equality checks parsing, not elaborated theorem equivalence; macros
that observe source positions are outside the supported contract. The CLI calls
Lean up to twice per input and prioritizes validation over speed. Only Linux has been
executed locally; the flake also declares aarch64-darwin.

Exit codes: 0 success, 252 invalid CLI grammar, 254 unsupported/invalid input,
I/O errors or a check finding changes, 255 unexpected failure. Diagnostics go
to stderr. `--debug` includes unexpected exception details.

For Emacs, the CLI can be used as a region filter with
`--output text source format-source --source file:///dev/stdin`; only replace
the buffer on exit 0. No running editor or user files are modified by setup.

Development uses uv, hatchling, pytest and ruff. `nix fmt` formats implementation
files; `make check` is the verification entry. Enable the optional pre-commit hook
with `git config core.hooksPath .githooks`.

Apache-2.0. The parser helper depends on Lean's public source APIs and is tested
with the Lean version pinned by flake.lock.
