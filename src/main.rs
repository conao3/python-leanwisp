use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use std::{
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Clone, Copy, ValueEnum)]
enum Output {
    Json,
    Text,
    Table,
}
#[derive(Parser)]
#[command(version, about = "Format Lean 4 source with parser-checked layout")]
struct Cli {
    #[arg(long, global = true, value_enum, default_value = "json")]
    output: Output,
    #[arg(long, global = true, overrides_with = "no_debug")]
    debug: bool,
    #[arg(long, global = true, overrides_with = "debug")]
    no_debug: bool,
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Source {
        #[command(subcommand)]
        command: Source,
    },
}
#[derive(Subcommand)]
enum Source {
    FormatSource(Options),
    CheckSource(Options),
}
#[derive(clap::Args)]
#[group(required = true, multiple = false)]
struct Input {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    source: Option<String>,
}
#[derive(clap::Args)]
struct Options {
    #[command(flatten)]
    input: Input,
    #[arg(long, overrides_with = "no_write")]
    write: bool,
    #[arg(long, overrides_with = "write")]
    no_write: bool,
    #[arg(long, default_value_t = 100)]
    line_width: usize,
    #[arg(long)]
    project_dir: Option<PathBuf>,
}
fn run(cli: Cli) -> Result<i32, String> {
    let Commands::Source { command } = cli.command;
    let check = matches!(command, Source::CheckSource(_));
    let (Source::FormatSource(opts) | Source::CheckSource(opts)) = command;
    if opts.write && (check || opts.input.file.is_none()) {
        return Err("--write requires format-source --file".into());
    }
    let file = opts.input.file;
    let source = if let Some(path) = &file {
        if path.is_symlink() {
            return Err("refusing a symlink input".into());
        }
        std::fs::read_to_string(path).map_err(|e| e.to_string())?
    } else {
        let value = opts.input.source.unwrap_or_default();
        if value == "file:///dev/stdin" {
            let mut data = String::new();
            std::io::stdin()
                .read_to_string(&mut data)
                .map_err(|e| e.to_string())?;
            data
        } else if let Some(path) = value.strip_prefix("file://") {
            std::fs::read_to_string(path).map_err(|e| e.to_string())?
        } else {
            value
        }
    };
    let project = opts.project_dir.or_else(|| {
        let start = file
            .as_ref()
            .and_then(|p| p.parent())
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        let start = std::fs::canonicalize(start).ok()?;
        start
            .ancestors()
            .find(|p| p.join("lakefile.toml").exists() || p.join("lakefile.lean").exists())
            .map(|p| p.to_owned())
    });
    let formatted = leanwisp::format_source_in(&source, opts.line_width, project.as_deref())?;
    let changed = source != formatted;
    if opts.write && changed {
        let path = file.ok_or("missing file")?;
        if std::fs::read_to_string(&path).map_err(|e| e.to_string())? != source {
            return Err("file changed during formatting".into());
        }
        std::fs::write(path, &formatted).map_err(|e| e.to_string())?;
    }
    match cli.output {
        Output::Json => println!("{}", json!({"Changed": changed, "Source": formatted})),
        Output::Text => std::io::stdout()
            .write_all(formatted.as_bytes())
            .map_err(|e| e.to_string())?,
        Output::Table => println!("Changed\n{changed}"),
    }
    Ok(if check && changed { 254 } else { 0 })
}
fn main() {
    let cli = Cli::try_parse().unwrap_or_else(|e| {
        let success = matches!(
            e.kind(),
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
        );
        let _ = e.print();
        std::process::exit(if success { 0 } else { 252 });
    });
    match run(cli) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(254);
        }
    }
}
