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
An in-workspace file read and hashed by Aporic is direct. The local CLI runner
can also produce a direct receipt for a pre-registered exact-argv specification;
MCP cannot execute checks or submit receipts. Receipts retain output hashes and
byte counts rather than raw stdout/stderr. External sources and user statements
remain reported, while model assessments remain model-only.

The runner is not a sandbox. Registered programs execute with the current user's
filesystem authority and a reduced environment, and timeout cleanup does not
guarantee containment of every descendant a hostile program may create. A
receipt proves the recorded process result and declared artifact hashes, not the
semantic quality of a test or a broader real-world effect. Do not run untrusted
specifications, or store secrets or raw conversation history in Aporic records.

## Reporting

Report suspected vulnerabilities privately to the repository maintainers. Do
not include secrets or exploit unrelated systems while preparing a report.
