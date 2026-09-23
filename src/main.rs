use aporic::analysis::{AnalyzerInput, DEFAULT_FUEL, DEFAULT_MEMORY_BYTES, run_wasm_analyzer};
use aporic::codex::{
    DEFAULT_PROJECTION_LIMIT_BYTES, GatePolicy, MAX_SCOPE_BYTES, MAX_USER_PROMPT_HOOK_INPUT_BYTES,
    PostToolUseInput, PreToolUseInput, SessionEndInput, SessionStartInput, UserPromptSubmitInput,
    claim_checkpoint_transaction, explain_action, invalid_policy_pre_tool_output,
    invalid_project_output, invalid_project_pre_tool_output, invalid_project_user_prompt_output,
    post_tool_use_transaction, pre_tool_use_transaction, publish_checkpoint_transaction,
    session_start_output, skipped_output, unavailable_output, unavailable_pre_tool_output,
    user_prompt_submit_output,
};
use aporic::mcp::serve_stdio;
use aporic::policy::PolicyDocument;
use aporic::project::{
    default_data_root, discover_project, initialize_project, store_path as project_store_path,
};
use aporic::verifier::{MAX_VERIFIER_REPORT_BYTES, VerifierReportInput, ingest_verifier_report};
use aporic::{
    CommitRequest, CommitStatus, PlanApprovalRequest, SCHEMA_VERSION, approve_plan, commit,
    initialize, load, load_nonblocking, migrate_to_current,
};
use serde::Serialize;
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

fn print_json(value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn read_bounded_stdin(
    limit_bytes: usize,
    input_kind: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    let mut stdin = io::stdin();
    {
        let mut limited = stdin.by_ref().take((limit_bytes + 1) as u64);
        limited.read_to_string(&mut input)?;
    }
    if input.len() > limit_bytes {
        io::copy(&mut stdin, &mut io::sink())?;
        return Err(format!("{input_kind} exceeds {limit_bytes} UTF-8 bytes").into());
    }
    Ok(input)
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

fn packaged_structural_analyzer_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let plugin_root = match env::var_os("PLUGIN_ROOT") {
        Some(root) => {
            let root = PathBuf::from(root);
            if !root.is_absolute() {
                return Err("PLUGIN_ROOT must be absolute".into());
            }
            root
        }
        None => env::current_exe()?
            .parent()
            .and_then(std::path::Path::parent)
            .ok_or("cannot resolve the package root from the executable path")?
            .to_path_buf(),
    };
    Ok(plugin_root.join("analyzers/structural.wasm"))
}

fn analyzer_limits(args: &[String]) -> Result<(u64, usize), Box<dyn std::error::Error>> {
    let fuel = optional_argument(args, "--fuel")?
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(DEFAULT_FUEL);
    let memory_bytes = optional_argument(args, "--memory-bytes")?
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(DEFAULT_MEMORY_BYTES);
    Ok((fuel, memory_bytes))
}

fn analyze(args: &[String], module: PathBuf) -> Result<i32, Box<dyn std::error::Error>> {
    let path = store_path(args)?;
    let scope = argument(args, "--scope")?;
    let (fuel, memory_bytes) = analyzer_limits(args)?;
    let log = load(path)?;
    let input = AnalyzerInput::from_state(log.state(), &scope);
    print_json(&run_wasm_analyzer(module, &input, fuel, memory_bytes)?)?;
    Ok(0)
}

fn data_root(args: &[String]) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = match optional_argument(args, "--data-root")? {
        Some(path) => PathBuf::from(path),
        None => default_data_root()?,
    };
    if !path.is_absolute() {
        return Err("data root must be absolute".into());
    }
    Ok(path)
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

fn gate_policy_from_path(path: &std::path::Path) -> Result<GatePolicy, Box<dyn std::error::Error>> {
    let document: PolicyDocument = serde_json::from_slice(&std::fs::read(path)?)?;
    GatePolicy::from_document(document).map_err(Into::into)
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
            "usage: aporic <mcp-serve|project-init|project-paths|init|status|commit|approve-plan|ingest-verifier-report|analyze-wasm|analyze-structural|explain|doctor|migrate|codex-session-start|codex-pre-tool-use|codex-post-tool-use|codex-session-end|codex-global-session-start|codex-global-user-prompt-submit|codex-global-pre-tool-use|codex-global-post-tool-use|codex-global-session-end>".into(),
        );
    };

    match command {
        "mcp-serve" => {
            serve_stdio()?;
            Ok(0)
        }
        "project-init" => {
            let workspace = PathBuf::from(argument(&args, "--workspace")?);
            let scope = argument(&args, "--scope")?;
            let migration_source = optional_argument(&args, "--migrate-from-v1-store")?;
            let data_root = data_root(&args)?;
            let canonical_workspace = std::fs::canonicalize(&workspace)?;
            let store = project_store_path(&data_root, &canonical_workspace)?;
            if store.try_exists()? {
                return Err("project store already exists; refusing to replace it".into());
            }
            let migration = if let Some(source) = migration_source {
                Some(migrate_to_current(source, &store, 1)?)
            } else {
                initialize(&store)?;
                None
            };
            let project = match initialize_project(workspace, scope) {
                Ok(project) => project,
                Err(error) => {
                    let _ = std::fs::remove_file(&store);
                    return Err(error.into());
                }
            };
            print_json(&serde_json::json!({
                "status": if migration.is_some() { "migrated" } else { "initialized" },
                "workspace": project.workspace,
                "scope": project.scope,
                "config": project.workspace.join(".aporic/config.json"),
                "policy": project.policy_path,
                "store": store,
                "migration": migration
            }))?;
            Ok(0)
        }
        "project-paths" => {
            let workspace = PathBuf::from(argument(&args, "--workspace")?);
            let project = discover_project(workspace, data_root(&args)?)?
                .ok_or("workspace is not bound to Aporic")?;
            print_json(&serde_json::json!({
                "workspace": project.workspace,
                "scope": project.scope,
                "policy": project.policy_path,
                "store": project.store_path
            }))?;
            Ok(0)
        }
        "init" => {
            let path = store_path(&args)?;
            initialize(path)?;
            print_json(&serde_json::json!({ "status": "initialized" }))?;
            Ok(0)
        }
        "status" => {
            let path = store_path(&args)?;
            print_json(load(path)?.state())?;
            Ok(0)
        }
        "commit" => {
            let path = store_path(&args)?;
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
        "approve-plan" => {
            let path = store_path(&args)?;
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let request: PlanApprovalRequest = serde_json::from_str(&input)?;
            let outcome = approve_plan(path, request)?;
            print_json(&outcome)?;
            Ok(if outcome.status == CommitStatus::Rejected {
                2
            } else {
                0
            })
        }
        "ingest-verifier-report" => {
            let path = store_path(&args)?;
            let scope = argument(&args, "--scope")?;
            let input = read_bounded_stdin(MAX_VERIFIER_REPORT_BYTES, "verifier report")?;
            let report: VerifierReportInput = serde_json::from_str(&input)?;
            ingest_verifier_report(path, &scope, &report)?;
            print_json(&serde_json::json!({
                "status": "accepted",
                "verification_id": report.verification_id
            }))?;
            Ok(0)
        }
        "analyze-wasm" => {
            let module = PathBuf::from(argument(&args, "--module")?);
            analyze(&args, module)
        }
        "analyze-structural" => analyze(&args, packaged_structural_analyzer_path()?),
        "migrate" => {
            let path = store_path(&args)?;
            let from = argument(&args, "--from")?;
            let from = from.parse::<u32>()?;
            let destination = PathBuf::from(argument(&args, "--to")?);
            print_json(&migrate_to_current(path, destination, from)?)?;
            Ok(0)
        }
        "codex-session-start" => {
            let path = store_path(&args)?;
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
            let output = match claim_checkpoint_transaction(&path, &input, &scope) {
                Ok(()) => match load_nonblocking(path) {
                    Ok(log) => session_start_output(
                        log.state(),
                        &input,
                        &scope,
                        &policy,
                        DEFAULT_PROJECTION_LIMIT_BYTES,
                    )
                    .map_err(String::from)?,
                    Err(error) => unavailable_output(&error, &input, &scope, &policy),
                },
                Err(error) => unavailable_output(&error, &input, &scope, &policy),
            };
            print_json(&output)?;
            Ok(0)
        }
        "codex-post-tool-use" => {
            let path = store_path(&args)?;
            let scope = argument(&args, "--scope")?;
            let workspace = PathBuf::from(argument(&args, "--workspace")?);
            let input = read_bounded_stdin(MAX_USER_PROMPT_HOOK_INPUT_BYTES, "hook input")?;
            let input: PostToolUseInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            if workspace_matches(&input.cwd, &workspace) {
                post_tool_use_transaction(path, &input, &scope)?;
            }
            Ok(0)
        }
        "codex-session-end" => {
            let path = store_path(&args)?;
            let scope = argument(&args, "--scope")?;
            let workspace = PathBuf::from(argument(&args, "--workspace")?);
            let input = read_bounded_stdin(MAX_USER_PROMPT_HOOK_INPUT_BYTES, "hook input")?;
            let input: SessionEndInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            if workspace_matches(&input.cwd, &workspace) {
                publish_checkpoint_transaction(path, &input, &scope)?;
            }
            Ok(0)
        }
        "codex-pre-tool-use" => {
            let path = store_path(&args)?;
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
        "codex-global-session-start" => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            let input: SessionStartInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            let data_root = match data_root(&args) {
                Ok(path) => path,
                Err(_) => {
                    print_json(&invalid_project_output(&input))?;
                    return Ok(0);
                }
            };
            let project = match discover_project(&input.cwd, &data_root) {
                Ok(Some(project)) => project,
                Ok(None) => {
                    print_json(&skipped_output())?;
                    return Ok(0);
                }
                Err(_) => {
                    print_json(&invalid_project_output(&input))?;
                    return Ok(0);
                }
            };
            let policy = match gate_policy_from_path(&project.policy_path) {
                Ok(policy) => policy,
                Err(_) => {
                    print_json(&invalid_project_output(&input))?;
                    return Ok(0);
                }
            };
            let output =
                match claim_checkpoint_transaction(&project.store_path, &input, &project.scope) {
                    Ok(()) => match load_nonblocking(&project.store_path) {
                        Ok(log) => session_start_output(
                            log.state(),
                            &input,
                            &project.scope,
                            &policy,
                            DEFAULT_PROJECTION_LIMIT_BYTES,
                        )
                        .map_err(String::from)?,
                        Err(error) => unavailable_output(&error, &input, &project.scope, &policy),
                    },
                    Err(error) => unavailable_output(&error, &input, &project.scope, &policy),
                };
            print_json(&output)?;
            Ok(0)
        }
        "codex-global-user-prompt-submit" => {
            let mut input = String::new();
            let mut stdin = io::stdin();
            {
                let mut limited = stdin
                    .by_ref()
                    .take((MAX_USER_PROMPT_HOOK_INPUT_BYTES + 1) as u64);
                limited.read_to_string(&mut input)?;
            }
            if input.len() > MAX_USER_PROMPT_HOOK_INPUT_BYTES {
                io::copy(&mut stdin, &mut io::sink())?;
                print_json(&skipped_output())?;
                return Ok(0);
            }
            let input: UserPromptSubmitInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            let data_root = match data_root(&args) {
                Ok(path) => path,
                Err(_) => {
                    print_json(&invalid_project_user_prompt_output(&input))?;
                    return Ok(0);
                }
            };
            match discover_project(&input.cwd, &data_root) {
                Ok(Some(_)) => {
                    print_json(&user_prompt_submit_output(&input).map_err(String::from)?)?;
                }
                Ok(None) => {
                    print_json(&skipped_output())?;
                }
                Err(_) => {
                    print_json(&invalid_project_user_prompt_output(&input))?;
                }
            }
            Ok(0)
        }
        "codex-global-pre-tool-use" => {
            let input = read_bounded_stdin(MAX_USER_PROMPT_HOOK_INPUT_BYTES, "hook input")?;
            let input: PreToolUseInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            let data_root = match data_root(&args) {
                Ok(path) => path,
                Err(_) => {
                    print_json(&invalid_project_pre_tool_output(&input.tool_name))?;
                    return Ok(0);
                }
            };
            let project = match discover_project(&input.cwd, &data_root) {
                Ok(Some(project)) => project,
                Ok(None) => return Ok(0),
                Err(_) => {
                    print_json(&invalid_project_pre_tool_output(&input.tool_name))?;
                    return Ok(0);
                }
            };
            let policy = match gate_policy_from_path(&project.policy_path) {
                Ok(policy) => policy,
                Err(_) => {
                    print_json(&invalid_policy_pre_tool_output(&input.tool_name))?;
                    return Ok(0);
                }
            };
            if policy.document().tool(&input.tool_name).is_none() {
                return Ok(0);
            }
            let output = match pre_tool_use_transaction(
                &project.store_path,
                &input,
                &project.scope,
                &policy,
            ) {
                Ok(output) => output,
                Err(error) => Some(unavailable_pre_tool_output(&error, &input.tool_name)),
            };
            if let Some(output) = output {
                print_json(&output)?;
            }
            Ok(0)
        }
        "codex-global-post-tool-use" => {
            let input = read_bounded_stdin(MAX_USER_PROMPT_HOOK_INPUT_BYTES, "hook input")?;
            let input: PostToolUseInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            let data_root = data_root(&args)?;
            let Some(project) = discover_project(&input.cwd, &data_root)? else {
                return Ok(0);
            };
            post_tool_use_transaction(&project.store_path, &input, &project.scope)?;
            Ok(0)
        }
        "codex-global-session-end" => {
            let input = read_bounded_stdin(MAX_USER_PROMPT_HOOK_INPUT_BYTES, "hook input")?;
            let input: SessionEndInput = serde_json::from_str(&input)?;
            input.validate().map_err(String::from)?;
            let data_root = data_root(&args)?;
            let Some(project) = discover_project(&input.cwd, &data_root)? else {
                return Ok(0);
            };
            publish_checkpoint_transaction(&project.store_path, &input, &project.scope)?;
            Ok(0)
        }
        "explain" => {
            let path = store_path(&args)?;
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
            let path = store_path(&args)?;
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
