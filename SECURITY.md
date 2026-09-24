# Security

Aporic is experimental software, not a hardened security boundary. Do not rely
on it as the sole control for untrusted code execution, secrets, production
deployment, authentication, or access control.

## Current development posture

This repository ships Rust crates, schemas, role packages, and an optional local
CLI. It does not install host hooks, ship an application plugin, intercept tool
calls, add approval prompts, or control model selection. Host and platform
permissions remain authoritative.

## Recorded state

The crates use bounded structured inputs, append-only records, deterministic
replay, revision checks, and cooperative file locks. These mechanisms protect
internal consistency; they do not authenticate callers or resist a malicious
process running as the same user. Connection files, role packages, and ledgers
should be protected with ordinary operating-system permissions.

Project bindings reject state placed inside the governed workspace and detect
changes to the bound Git remote. Continuity and effect records are observations,
not proof that an external effect occurred.

Markdown role instructions are hashed and carried as behavioral context. They
are not parsed by the deterministic kernel and do not grant authority.

## Reporting

Report suspected vulnerabilities privately to the repository maintainers. Do
not include secrets or exploit unrelated systems while preparing a report.
