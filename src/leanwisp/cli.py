"""CLI and byte-preserving whitespace edits."""

import argparse
import itertools
import json
import subprocess
import sys
from pathlib import Path


class InputError(Exception):
    """Input outside the supported Lean subset."""


def inspect(source: str) -> dict:
    proc = subprocess.run(
        ["lean", "--run", str(Path(__file__).with_name("Inspect.lean"))],
        check=False,
        input=source,
        text=True,
        capture_output=True,
        timeout=60,
    )
    if proc.returncode:
        raise InputError(proc.stdout + proc.stderr)
    return json.loads(proc.stdout)


def format_source(source: str) -> str:
    original = inspect(source)
    data = source.encode("utf-8")
    edits = {}
    tokens = original["Tokens"]
    spaced = {":", ":=", "=>", "→", "->", "←", "=", "+", "*"}
    for left, right in itertools.pairwise(tokens):
        a, b = left[1], right[0]
        gap = data[a:b]
        if any(c not in b" \t" for c in gap):
            continue
        # Preserve indentation, comments, strings and all line boundaries.
        if not left[2] and a == left[0]:
            continue
        if left[2] in spaced or right[2] in spaced or left[2] == ",":
            edits[a, b] = b" "
    for (a, b), replacement in sorted(edits.items(), reverse=True):
        data = data[:a] + replacement + data[b:]
    result = data.decode("utf-8")
    if result != source and inspect(result)["Tree"] != original["Tree"]:
        raise InputError(
            "formatting would change the Lean syntax tree; input unchanged"
        )
    return result


class Parser(argparse.ArgumentParser):
    def error(self, message):
        self.print_usage(sys.stderr)
        self.exit(252, f"{message}\n")


def parser():
    root = Parser(prog="leanwisp")
    root.add_argument("--version", action="version", version="leanwisp 0.1.0")
    root.add_argument("--output", choices=["json", "text", "table"], default="json")
    root.add_argument("--debug", action=argparse.BooleanOptionalAction, default=False)
    commands = root.add_subparsers(dest="command", required=True)
    source = commands.add_parser("source")
    verbs = source.add_subparsers(dest="operation", required=True)
    for name in ("format-source", "check-source"):
        verb = verbs.add_parser(name)
        inputs = verb.add_mutually_exclusive_group(required=True)
        inputs.add_argument(
            "--source",
            help="literal source or file:// path (file:///dev/stdin for stdin)",
        )
        inputs.add_argument("--file", type=Path)
        write = verb.add_mutually_exclusive_group()
        write.add_argument("--write", action="store_true")
        write.add_argument("--no-write", dest="write", action="store_false")
        verb.set_defaults(write=False)
    return root


def main():
    argsv = sys.argv[1:]
    for prefix in (
        ["help"],
        ["source", "help"],
        ["source", "format-source", "help"],
        ["source", "check-source", "help"],
    ):
        if argsv == prefix:
            argsv = prefix[:-1] + ["--help"]
            break
    args = parser().parse_args(argsv)
    try:
        if args.write and (args.file is None or args.operation != "format-source"):
            raise InputError("--write requires format-source --file")
        if args.file:
            if args.file.is_symlink():
                raise InputError("symlink inputs are unsupported")
            data = args.file.read_bytes().decode("utf-8")
        elif args.source.startswith("file://"):
            data = Path(args.source[7:]).read_bytes().decode("utf-8")
        else:
            data = args.source
        result = format_source(data)
        changed = result != data
        if args.write and changed:
            # Recheck before writing to avoid overwriting concurrent edits.
            if args.file.read_bytes().decode("utf-8") != data:
                raise InputError("file changed during formatting")
            args.file.write_bytes(result.encode("utf-8"))
        response = {"Changed": changed, "Source": result}
        if args.output == "text":
            sys.stdout.write(result)
        elif args.output == "table":
            print("Changed\n" + str(changed).lower())
        else:
            print(json.dumps(response, ensure_ascii=False))
        return 254 if changed and args.operation == "check-source" else 0
    except (InputError, OSError, UnicodeError, subprocess.TimeoutExpired) as exc:
        print(str(exc), file=sys.stderr)
        return 254
    except Exception:
        if args.debug:
            raise
        print("unexpected formatter failure", file=sys.stderr)
        return 255
