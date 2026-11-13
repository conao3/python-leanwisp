use leanwisp::format_source;

fn stable(source: &str, width: usize) -> String {
    let result = format_source(source, width).unwrap();
    assert_eq!(format_source(&result, width).unwrap(), result);
    result
}

#[test]
fn declaration_layout() {
    let result = stable(
        "import Std\ndef double(n:Nat):Nat:=n+n\ndef answer:Nat:=42\n",
        100,
    );
    assert!(
        result.contains("def double (n : Nat) : Nat :=\n  n + n"),
        "{result}"
    );
    assert!(result.contains("\n\ndef answer"), "{result}");
}

#[test]
fn proofs_do_match_structures() {
    for source in [
        "example (p q : Prop) (hp : p) (hq : q) : p ∧ q := by\n     constructor\n     · exact hp\n     · exact hq\n",
        "def action : IO Unit := do\n    let x := 1\n    IO.println x\n",
        "def f (x : Nat) : Nat :=\n match x with\n | 0 => 1\n | n+1 => n\n",
        "structure Person where\n    name : String\n    age : Nat\n",
        "namespace Foo\ndef x:Nat:=3\nend Foo\n",
    ] {
        stable(source, 80);
    }
}

#[test]
fn width_changes_layout() {
    let source = "def values : List Nat := [111111, 222222, 333333, 444444, 555555, 666666]\n";
    let wide = stable(source, 100);
    let narrow = stable(source, 35);
    assert_ne!(wide, narrow);
    assert!(narrow.lines().count() > wide.lines().count(), "{narrow}");
}

#[test]
fn comment_and_literal_contents() {
    for source in [
        "-- before\ndef x:Nat:=3 -- after\n-- eof\n",
        "/- outer /- inner -/ unchanged -/\ndef x : Nat := 3\n",
        "def s : String := \"not -- a comment : :=\"\n",
        "def s : String := r#\"raw : -- /- text\"#\n",
        "def c : Char := ':'\n",
        "/-- Documentation. -/\ndef «a:b» : Nat := 3\n",
        "def s : String := \"first\n  second\"\n",
        "def s := s!\"result {(3:Nat)}\"\n",
    ] {
        stable(source, 80);
    }
}

#[test]
fn imported_and_local_notation() {
    stable("import Lean\ndef x : Nat := 3\n", 100);
    let result = stable("infixl:65 \" +++ \" => Nat.add\n#check 1+++2\n", 80);
    assert!(result.contains("+++"));
}

#[test]
fn practice_and_large_fixture() {
    for source in [
        include_str!("fixtures/Basics.lean"),
        include_str!("fixtures/PracticeBasics.lean"),
        include_str!("fixtures/PracticeProofs.lean"),
    ] {
        stable(source, 80);
    }
}

#[test]
fn invalid_input_is_rejected() {
    for source in ["#eval (", "/- unclosed", "import Missing.Leanwisp.Module\n"] {
        assert!(format_source(source, 80).is_err());
    }
    assert!(format_source("#check Nat", 5).is_err());
}

#[test]
fn canonical_golden() {
    assert_eq!(
        stable("def double(n:Nat):Nat:=n+n\ndef answer:Nat:=42", 100),
        "def double (n : Nat) : Nat :=\n  n + n\n\ndef answer : Nat :=\n  42\n"
    );
}

#[test]
fn long_module() {
    let source: String = (0..150)
        .map(|i| format!("def number{i}:Nat:={i}\n"))
        .collect();
    let result = stable(&source, 80);
    assert_eq!(result.matches("def number").count(), 150);
}

#[test]
fn compile_formatted_fixtures() {
    let dir = std::env::temp_dir().join(format!("leanwisp-build-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, source) in [
        ("Basics", include_str!("fixtures/Basics.lean")),
        ("Proofs", include_str!("fixtures/PracticeProofs.lean")),
    ] {
        let file = dir.join(format!("{name}.lean"));
        std::fs::write(&file, stable(source, 80)).unwrap();
        let output = std::process::Command::new("lean")
            .arg(&file)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cli_and_lake_project() {
    use std::process::Command;
    let dir = std::env::temp_dir().join(format!("leanwisp-project-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("lakefile.toml"),
        "name = \"fixture\"\n[[lean_lib]]\nname = \"Support\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Support.lean"),
        "notation \"answer!\" => (42 : Nat)\n",
    )
    .unwrap();
    let build = Command::new("lake")
        .args(["build", "Support"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let file = dir.join("Example.lean");
    let source = "import Support\n#check answer!\ndef x:Nat:=3\n";
    std::fs::write(&file, source).unwrap();
    let binary = env!("CARGO_BIN_EXE_leanwisp");
    let check = Command::new(binary)
        .args(["source", "check-source", "--file"])
        .arg(&file)
        .output()
        .unwrap();
    assert_eq!(
        check.status.code(),
        Some(254),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), source);
    let write = Command::new(binary)
        .args(["source", "format-source", "--file"])
        .arg(&file)
        .arg("--write")
        .output()
        .unwrap();
    assert!(
        write.status.success(),
        "{}",
        String::from_utf8_lossy(&write.stderr)
    );
    let check = Command::new(binary)
        .args(["source", "check-source", "--file"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let relative = Command::new(binary)
        .args(["source", "check-source", "--file", "Example.lean"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        relative.status.success(),
        "{}",
        String::from_utf8_lossy(&relative.stderr)
    );
    for args in [
        vec!["help"],
        vec!["source", "help"],
        vec!["source", "format-source", "--help"],
    ] {
        assert!(
            Command::new(binary)
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    assert_eq!(
        Command::new(binary)
            .args(["source", "format-source"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(252)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn proof_and_comment_golden() {
    let source = "-- before\nexample (p q:Prop) (hp:p) (hq:q):p ∧ q:=by\n    constructor\n    · exact hp -- first\n    · exact hq\n-- after\n";
    let result = stable(source, 100);
    assert!(result.contains(":= by\n  constructor"), "{result}");
    for comment in ["-- before", "-- first", "-- after"] {
        assert_eq!(result.matches(comment).count(), 1, "{result}");
    }
    assert!(result.contains("  · exact hp"), "{result}");
}

#[test]
fn formatter_adapter_source() {
    stable(include_str!("../lean/Format.lean"), 100);
}

#[test]
fn exit_does_not_drop_following_source() {
    let result = stable(
        "#exit\n-- keep the following code\ndef later:Nat:=42\n",
        100,
    );
    assert!(result.contains("def later : Nat"), "{result}");
    assert!(result.contains("-- keep the following code"), "{result}");
}
