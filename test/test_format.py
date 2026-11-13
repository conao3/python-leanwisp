import json
import subprocess
from pathlib import Path

import pytest

from leanwisp.cli import InputError, format_source

FIXTURE = Path(__file__).parent / "fixtures" / "Basics.lean"


def test_fixture_build_and_idempotence(tmp_path):
    source = FIXTURE.read_text()
    result = format_source(source)
    assert "(3 : Nat)" in result
    assert "def double (n : Nat) : Nat := n + n" in result
    assert '"colon: inside string := untouched"' in result
    assert "«a:b»" in result
    assert "/- outer /- inner : := -/ end -/" in result
    assert result.count("\n") == source.count("\n")
    assert format_source(result) == result
    for name, value in [("Before", source), ("After", result)]:
        path = tmp_path / f"{name}.lean"
        path.write_text(value)
        proc = subprocess.run(
            ["lean", str(path)], check=False, capture_output=True, text=True
        )
        assert proc.returncode == 0, proc.stdout + proc.stderr


@pytest.mark.parametrize(
    "source",
    [
        "#eval (3:Nat)",
        "#eval (3:Nat)\r\n",
        "import Init\n#eval (3:Nat)\n",
        "example : True := by\n  exact\n    True.intro\n",
        '#eval r#"raw: := string"#\n',
        '#eval s!"interpolation {(3:Nat)}"\n',
        "-- trailing comment: :=\n#eval (3:Nat) -- keep\n",
    ],
)
def test_special_cases(source):
    result = format_source(source)
    assert format_source(result) == result
    assert result.count("\r\n") == source.count("\r\n")


@pytest.mark.parametrize(
    "source",
    [
        "import Lean\n#eval 3\n",
        'syntax "foo" : term\n',
        'infixl:65 " +++ " => Nat.add\n',
        "#eval (\n",
        "/- unclosed",
        '#eval "unclosed',
    ],
)
def test_unsupported(source):
    with pytest.raises(InputError):
        format_source(source)


def run(*args, input=None):
    return subprocess.run(
        ["leanwisp", *args], check=False, input=input, text=True, capture_output=True
    )


def test_cli(tmp_path):
    proc = run(
        "source",
        "format-source",
        "--source",
        "file:///dev/stdin",
        input="#eval (3:Nat)\n",
    )
    assert proc.returncode == 0, proc.stderr
    assert json.loads(proc.stdout)["Source"] == "#eval (3 : Nat)\n"
    path = tmp_path / "Input.lean"
    path.write_text("#eval (3:Nat)\n")
    assert run("source", "check-source", "--file", str(path)).returncode == 254
    assert path.read_text() == "#eval (3:Nat)\n"
    assert (
        run("source", "format-source", "--file", str(path), "--write").returncode == 0
    )
    assert run("source", "check-source", "--file", str(path)).returncode == 0
    path.write_text("import Lean\n")
    assert (
        run("source", "format-source", "--file", str(path), "--write").returncode == 254
    )
    assert path.read_text() == "import Lean\n"
    assert run("source", "format-source").returncode == 252
    for args in [("help",), ("source", "help"), ("source", "format-source", "help")]:
        assert run(*args).returncode == 0


@pytest.mark.parametrize("name", ["PracticeBasics", "PracticeProofs"])
def test_practice_projects(name, tmp_path):
    source = FIXTURE.with_name(f"{name}.lean").read_text()
    result = format_source(source)
    assert format_source(result) == result
    path = tmp_path / f"{name}.lean"
    path.write_text(result)
    proc = subprocess.run(
        ["lean", str(path)], capture_output=True, text=True, check=False
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr
