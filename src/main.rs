use aporic::codex::{
    DEFAULT_CONTEXT_LIMIT_BYTES, MAX_SCOPE_BYTES, PreToolUseInput, SessionStartInput,
    pre_tool_use_output, session_start_output, skipped_output, unavailable_output,
    unavailable_pre_tool_output,
};
use aporic::{CommitRequest, CommitStatus, commit, initialize, load, load_nonblocking};
use serde::Serialize;
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

fn print_json(value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn store_path(args: &[String]) -> Result<PathBuf, String> {
    let Some(index) = args.iter().position(|arg| arg == "--store") else {
        return Err("missing required --store <events.jsonl>".into());
    };
    args.get(index + 1)
        .map(PathBuf::from)
        .ok_or_else(|| "missing value after --store".into())
}

fn argument(args: &[String], name: &str) -> Result<String, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Err(format!("missing required {name} <value>"));
    };
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("missing value after {name}"))
}

fn workspace_matches(cwd: &str, workspace: &PathBuf) -> bool {
    matches!(
        (std::fs::canonicalize(cwd), std::fs::canonicalize(workspace)),
        (Ok(current), Ok(configured)) if current == configured
    )
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        return Err(
            "usage: aporic <init|status|commit|codex-session-start|codex-pre-tool-use> --store <events.jsonl>".into(),
        );
    };
    let path = store_path(&args)?;

    match command {
        "init" => {
            initialize(path)?;
            print_json(&serde_json::json!({ "status": "initialized" }))?;
            Ok(0)
        }
        "status" => {
            print_json(load(path)?.state())?;
            Ok(0)
        }
        "commit" => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let request: CommitRequest = serde_json::from_str(&input)?;
            let outcome = commit(path, request)?;
            print_json(&outcome)?;
            Ok(if outcome.status == CommitStatus::Rejected {
                2
            } else {
                0
            })
        }
        "codex-session-start" => {
            let scope = argument(&args, "--scope")?;
            let workspace = PathBuf::from(argument(&args, "--workspace")?);
            if scope.trim().is_empty() || scope.len() > MAX_SCOPE_BYTES {
                return Err(format!("scope must be 1..={MAX_SCOPE_BYTES} UTF-8 bytes").into());
            }
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let input: SessionStartInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            if !workspace_matches(&input.cwd, &workspace) {
                print_json(&skipped_output())?;
                return Ok(0);
            }
            let output = match load(path) {
                Ok(log) => {
                    session_start_output(log.state(), &input, &scope, DEFAULT_CONTEXT_LIMIT_BYTES)
                        .map_err(String::from)?
                }
                Err(error) => unavailable_output(&error, &scope),
            };
            print_json(&output)?;
            Ok(0)
        }
        "codex-pre-tool-use" => {
            let scope = argument(&args, "--scope")?;
            let workspace = PathBuf::from(argument(&args, "--workspace")?);
            let protected_tool = argument(&args, "--protected-tool")?;
            let require_plan = args.iter().any(|arg| arg == "--require-plan");
            if scope.trim().is_empty() || scope.len() > MAX_SCOPE_BYTES {
                return Err(format!("scope must be 1..={MAX_SCOPE_BYTES} UTF-8 bytes").into());
            }
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let input: PreToolUseInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            if !workspace_matches(&input.cwd, &workspace) || input.tool_name != protected_tool {
                return Ok(0);
            }
            let output = match load_nonblocking(path) {
                Ok(log) => {
                    pre_tool_use_output(log.state(), &input, &scope, &protected_tool, require_plan)
                        .map_err(String::from)?
                }
                Err(error) => Some(unavailable_pre_tool_output(&error, &input.tool_name)),
            };
            if let Some(output) = output {
                print_json(&output)?;
            }
            Ok(0)
        }
        _ => Err(format!("unknown command {command:?}").into()),
    }
}

fn main() {
    match run() {
        Ok(code) if code != 0 => std::process::exit(code),
        Ok(_) => {}
        Err(error) => {
            eprintln!("aporic: {error}");
            std::process::exit(1);
        }
    }
}
