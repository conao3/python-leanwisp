use serde::Deserialize;
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Parsed {
    tree: Value,
    document: Value,
    trivia: Vec<String>,
}

fn comments(trivia: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for text in trivia {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            let start = i;
            if &bytes[i..i + 2] == b"--" {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                result.push(text[start..i].trim_end_matches('\r').to_owned());
            } else if &bytes[i..i + 2] == b"/-" {
                let mut depth = 1;
                i += 2;
                while i + 1 < bytes.len() && depth > 0 {
                    if &bytes[i..i + 2] == b"/-" {
                        depth += 1;
                        i += 2;
                    } else if &bytes[i..i + 2] == b"-/" {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                result.push(text[start..i].to_owned());
            } else {
                i += 1;
            }
        }
    }
    result
}

fn parse(source: &str, project: Option<&std::path::Path>) -> Result<Parsed, String> {
    let mut command = if let Some(binary) = std::env::var_os("LEANWISP_SYNTAX") {
        Command::new(binary)
    } else {
        let mut cmd = Command::new("lean");
        cmd.args([
            "--run",
            concat!(env!("CARGO_MANIFEST_DIR"), "/lean/Format.lean"),
        ]);
        cmd
    };
    if let Some(dir) = project {
        let program = command.get_program().to_owned();
        let args: Vec<_> = command.get_args().map(|s| s.to_owned()).collect();
        let mut lake = Command::new("lake");
        lake.arg("env").arg(program).args(args).current_dir(dir);
        command = lake;
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // Ensure a killed editor/filter does not leave a parser process behind.
        unsafe {
            command.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() == 1 {
                    libc::raise(libc::SIGKILL);
                }
                Ok(())
            });
        }
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start Lean parser: {e}"))?;
    // Read output concurrently with writing: large documents can exceed pipe capacity.
    let mut stdin = child.stdin.take().ok_or("missing parser stdin")?;
    let source = source.to_owned();
    let writer = std::thread::spawn(move || stdin.write_all(source.as_bytes()));
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let written = writer.join().map_err(|_| "parser writer panicked")?;
    if !output.status.success() {
        return Err(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    written.map_err(|e| e.to_string())?;
    {
        let mut de = serde_json::Deserializer::from_slice(&output.stdout);
        de.disable_recursion_limit();
        Parsed::deserialize(&mut de).map_err(|e| format!("invalid parser response: {e}"))
    }
}

#[derive(Clone, Copy)]
struct Frame<'a> {
    indent: i64,
    flat: bool,
    fill: bool,
    doc: &'a Value,
}

fn tag(doc: &Value) -> &str {
    doc[0].as_str().unwrap_or("nil")
}

fn fits(mut remaining: i64, mut stack: Vec<Frame<'_>>) -> bool {
    while remaining >= 0 {
        let Some(frame) = stack.pop() else {
            return true;
        };
        let d = frame.doc;
        match tag(d) {
            "text" => {
                let text = d[1].as_str().unwrap_or("");
                if text.contains('\n') {
                    return !frame.flat;
                }
                remaining -= text.chars().count() as i64;
            }
            "line" if !frame.flat => return true,
            "line" => remaining -= 1,
            "append" => {
                stack.push(Frame {
                    doc: &d[2],
                    ..frame
                });
                stack.push(Frame {
                    doc: &d[1],
                    ..frame
                });
            }
            "nest" => stack.push(Frame {
                doc: &d[2],
                ..frame
            }),
            "group" => stack.push(Frame {
                doc: &d[1],
                ..frame
            }),
            _ => {}
        }
    }
    false
}

/// Wadler-style document layout. Literal text is emitted verbatim, including
/// multiline comments and strings; only document line nodes introduce indentation.
pub fn render(doc: &Value, width: usize) -> String {
    let mut stack = vec![Frame {
        indent: 0,
        flat: false,
        fill: false,
        doc,
    }];
    let mut out = String::new();
    let mut column = 0;
    let mut pending = 0;
    let mut braces: Vec<usize> = Vec::new();
    while let Some(frame) = stack.pop() {
        let d = frame.doc;
        match tag(d) {
            "text" => {
                let text = d[1].as_str().unwrap_or("");
                if text.is_empty() {
                    continue;
                }
                if text.chars().all(char::is_whitespace) && text.contains('\n') {
                    for _ in 0..text.chars().filter(|c| *c == '\n').count() {
                        if !out.ends_with("\n\n") {
                            out.push('\n');
                        }
                    }
                    column = 0;
                    pending = frame.indent.max(0) as usize;
                    continue;
                }
                // Keep block introducers with their assignment. Lean's document
                // can place `by` on a separate line when its proof does not flatten.
                if matches!(text, "by" | "do") && out.trim_end().ends_with(":=") {
                    out.truncate(out.trim_end().len());
                    out.push(' ');
                    pending = 0;
                    column = out.rsplit('\n').next().unwrap_or("").chars().count();
                }
                if text.trim() == "}" {
                    braces.pop();
                }
                // A record that starts after an application on the same line
                // must keep later fields beyond its opening brace. Otherwise
                // Lean interprets the wrapped field as the end of the record.
                if column == 0 && text.trim() != "}" {
                    pending = pending.max(braces.last().copied().unwrap_or(0));
                }
                if pending > 0 && !text.starts_with('\n') {
                    out.extend(std::iter::repeat_n(' ', pending));
                    column = pending;
                }
                pending = 0;
                out.push_str(text);
                column = text
                    .rsplit_once('\n')
                    .map_or(column + text.chars().count(), |(_, tail)| {
                        tail.chars().count()
                    });
                if text.trim() == "{" {
                    braces.push(column + 1);
                }
            }
            "line"
                if frame.flat
                    || (frame.fill
                        && fits(width.saturating_sub(column + 1) as i64, stack.clone())) =>
            {
                out.push(' ');
                column += 1;
            }
            "line" => {
                if !out.ends_with("\n\n") {
                    out.push('\n');
                }
                column = 0;
                pending = frame.indent.max(0) as usize;
            }
            "align" => {
                let indent = frame.indent.max(0) as usize;
                if (!frame.flat || d[1].as_bool().unwrap_or(false)) && column >= indent {
                    out.push('\n');
                    column = 0;
                    pending = indent;
                } else if column < indent {
                    out.extend(std::iter::repeat_n(' ', indent - column));
                    column = indent;
                }
            }
            "append" => {
                stack.push(Frame {
                    doc: &d[2],
                    ..frame
                });
                stack.push(Frame {
                    doc: &d[1],
                    ..frame
                });
            }
            "nest" => stack.push(Frame {
                indent: frame.indent + d[1].as_i64().unwrap_or(0),
                doc: &d[2],
                ..frame
            }),
            "group" => {
                let trial = vec![Frame {
                    flat: true,
                    doc: &d[1],
                    ..frame
                }];
                let flat =
                    frame.flat || fits(width.saturating_sub(column.max(pending)) as i64, trial);
                stack.push(Frame {
                    flat,
                    fill: !flat && d[2].as_bool().unwrap_or(false),
                    doc: &d[1],
                    ..frame
                });
            }
            _ => {}
        }
    }
    out.trim_matches([' ', '\n', '\r', '\t']).to_owned() + "\n"
}

pub fn format_source(source: &str, width: usize) -> Result<String, String> {
    format_source_in(source, width, None)
}

pub fn format_source_in(
    source: &str,
    width: usize,
    project: Option<&std::path::Path>,
) -> Result<String, String> {
    if !(20..=300).contains(&width) {
        return Err("line width must be between 20 and 300".into());
    }
    let original = parse(source, project)?;
    let formatted = render(&original.document, width);
    let checked = parse(&formatted, project)?;
    if original.tree != checked.tree {
        return Err("formatting would change the Lean syntax tree; input unchanged".into());
    }
    if comments(&original.trivia) != comments(&checked.trivia) {
        return Err("formatting would change comment contents or ordering; input unchanged".into());
    }
    // A formatter must settle after one pass, including its handling of comments.
    if render(&checked.document, width) != formatted {
        return Err("formatter did not reach a stable layout; input unchanged".into());
    }
    Ok(formatted)
}
