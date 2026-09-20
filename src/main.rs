use aporic::codex::{
    DEFAULT_PROJECTION_LIMIT_BYTES, GatePolicy, MAX_SCOPE_BYTES, PreToolUseInput,
    SessionStartInput, explain_action, invalid_policy_pre_tool_output, pre_tool_use_transaction,
    session_start_output, skipped_output, unavailable_output, unavailable_pre_tool_output,
};
use aporic::policy::PolicyDocument;
use aporic::{
    CommitRequest, CommitStatus, SCHEMA_VERSION, commit, initialize, load, load_nonblocking,
    migrate_v1_to_v2,
};
use serde::Serialize;
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

fn print_json(value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn store_path(args: &[String]) -> Result<PathBuf, String> {
    argument(args, "--store").map(PathBuf::from)
}

fn argument(args: &[String], name: &str) -> Result<String, String> {
    let mut matches = args
        .iter()
        .enumerate()
        .filter(|(_, argument)| argument.as_str() == name);
    let Some((index, _)) = matches.next() else {
        return Err(format!("missing required {name} <value>"));
    };
    if matches.next().is_some() {
        return Err(format!("duplicate {name} option"));
    }
    let value = args
        .get(index + 1)
        .cloned()
        .ok_or_else(|| format!("missing value after {name}"))?;
    if value.starts_with("--") {
        return Err(format!("missing value after {name}"));
    }
    Ok(value)
}

fn optional_argument(args: &[String], name: &str) -> Result<Option<String>, String> {
    if !args.iter().any(|argument| argument == name) {
        return Ok(None);
    }
    argument(args, name).map(Some)
}

fn gate_policy(args: &[String]) -> Result<GatePolicy, Box<dyn std::error::Error>> {
    let policy_path = optional_argument(args, "--policy")?;
    let protected_tool = optional_argument(args, "--protected-tool")?;
    if policy_path.is_some() && protected_tool.is_some() {
        return Err("--policy and --protected-tool are mutually exclusive".into());
    }
    if let Some(path) = policy_path {
        if args.iter().any(|argument| argument == "--require-plan") {
            return Err("--require-plan is only valid with legacy --protected-tool".into());
        }
        let bytes = std::fs::read(path)?;
        let document: PolicyDocument = serde_json::from_slice(&bytes)?;
        return GatePolicy::from_document(document).map_err(Into::into);
    }
    let protected_tool =
        protected_tool.ok_or("missing required --policy <path> or --protected-tool <name>")?;
    let require_plan = args.iter().any(|arg| arg == "--require-plan");
    GatePolicy::new(protected_tool, require_plan)
        .map_err(String::from)
        .map_err(Into::into)
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
            "usage: aporic <init|status|commit|explain|doctor|migrate|codex-session-start|codex-pre-tool-use> --store <events.jsonl>".into(),
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
        "migrate" => {
            let from = argument(&args, "--from")?;
            if from != "1" {
                return Err("only --from 1 is supported".into());
            }
            let destination = PathBuf::from(argument(&args, "--to")?);
            print_json(&migrate_v1_to_v2(path, destination)?)?;
            Ok(0)
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
            let policy = gate_policy(&args)?;
            let output = match load_nonblocking(path) {
                Ok(log) => session_start_output(
                    log.state(),
                    &input,
                    &scope,
                    &policy,
                    DEFAULT_PROJECTION_LIMIT_BYTES,
                )
                .map_err(String::from)?,
                Err(error) => unavailable_output(&error, &input, &scope, &policy),
            };
            print_json(&output)?;
            Ok(0)
        }
        "codex-pre-tool-use" => {
            let scope = argument(&args, "--scope")?;
            let workspace = PathBuf::from(argument(&args, "--workspace")?);
            if scope.trim().is_empty() || scope.len() > MAX_SCOPE_BYTES {
                return Err(format!("scope must be 1..={MAX_SCOPE_BYTES} UTF-8 bytes").into());
            }
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let input: PreToolUseInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            if !workspace_matches(&input.cwd, &workspace) {
                return Ok(0);
            }
            let policy = match gate_policy(&args) {
                Ok(policy) => policy,
                Err(_) => {
                    print_json(&invalid_policy_pre_tool_output(&input.tool_name))?;
                    return Ok(0);
                }
            };
            if policy.document().tool(&input.tool_name).is_none() {
                return Ok(0);
            }
            let output = match pre_tool_use_transaction(&path, &input, &scope, &policy) {
                Ok(output) => output,
                Err(error) => Some(unavailable_pre_tool_output(&error, &input.tool_name)),
            };
            if let Some(output) = output {
                print_json(&output)?;
            }
            Ok(0)
        }
        "explain" => {
            let scope = argument(&args, "--scope")?;
            let policy = gate_policy(&args)?;
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let input: PreToolUseInput = serde_json::from_str(&input)?;
            let log = load(path)?;
            print_json(
                &explain_action(log.state(), &input, &scope, &policy).map_err(String::from)?,
            )?;
            Ok(0)
        }
        "doctor" => {
            let policy = match gate_policy(&args) {
                Ok(policy) => policy,
                Err(error) => {
                    print_json(&serde_json::json!({
                        "healthy": false,
                        "event_schema": SCHEMA_VERSION,
                        "error": error.to_string(),
                        "checks": []
                    }))?;
                    return Ok(2);
                }
            };
            match load_nonblocking(path) {
                Ok(log) => {
                    print_json(&serde_json::json!({
                        "healthy": true,
                        "event_schema": SCHEMA_VERSION,
                        "revision": log.state().revision,
                        "protected_tools": policy.document().tools.keys().collect::<Vec<_>>(),
                        "checks": ["policy_valid", "store_readable", "log_replay_valid"]
                    }))?;
                    Ok(0)
                }
                Err(error) => {
                    print_json(&serde_json::json!({
                        "healthy": false,
                        "event_schema": SCHEMA_VERSION,
                        "error": error.to_string(),
                        "checks": ["policy_valid"]
                    }))?;
                    Ok(2)
                }
            }
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
