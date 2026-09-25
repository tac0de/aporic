# Security

Aporic is experimental software, not a hardened security boundary. Do not rely
on it as the sole control for untrusted code execution, secrets, production
deployment, authentication, or access control.

## Current development posture

This repository ships one Rust package with a local stdio MCP server and CLI.
It does not install host hooks, intercept tool calls, add approval prompts,
launch workers, or grant host authority. Model routing is a recommendation only;
host and platform permissions remain authoritative.

## Recorded state

The hub uses bounded structured inputs, append-only events, idempotency keys,
SQLite transactions, and deterministic projections. These mechanisms protect
internal consistency; they do not authenticate callers or resist a malicious
process running as the same user. Protect the application-data database with
ordinary operating-system permissions.

Runtime state stays outside governed workspaces and `~/.codex`. Task leases and
write scopes are advisory coordination records, not locks on the filesystem.
Only an in-workspace file read and hashed by Aporic is classified as direct.
Command results, external sources, and user statements are reported; model
assessments are model-only. This blocks those sources from independently
establishing verified completion, but it does not prove that file contents imply
a broader real-world effect. Do not store secrets or raw conversation history in
Aporic records.

## Reporting

Report suspected vulnerabilities privately to the repository maintainers. Do
not include secrets or exploit unrelated systems while preparing a report.
