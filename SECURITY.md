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
Scopes are conservatively canonicalized to lower-case portable ASCII so aliases
compare consistently across supported filesystems; use a shared ASCII parent
scope when work includes non-ASCII filenames.
An in-workspace file read and hashed by Aporic is direct. The local CLI runner
can also produce a direct receipt for a pre-registered exact-argv specification;
MCP cannot execute checks or submit receipts. Receipts retain output hashes and
byte counts rather than raw stdout/stderr. External sources and user statements
remain reported, while model assessments remain model-only.

The default `host` runner profile is not a sandbox. Registered programs execute
with the current user's filesystem authority and a reduced environment, and
timeout cleanup does not guarantee containment of every descendant a hostile
program may create.

The opt-in `required` profile is available only when Aporic can start Linux
`bubblewrap` and the host permits its user-namespace setup. It isolates
namespaces, drops capabilities, exposes only a minimal read-only system view,
gives the process a private home and `/tmp`, denies network access, and mounts
the workspace read-only or read-write as declared.
The child must write a readiness marker from inside that mount namespace before
its exit can be evaluated. Missing support, setup failure, or missing readiness
evidence fails closed and never falls back to `host`.

This profile materially narrows accidental and adversarial filesystem/network
reach, but it is not a hardened multi-tenant boundary. It has no cgroup CPU,
memory, or process-count quota, no Aporic-specific seccomp allowlist, and no
VM/microVM isolation. Kernel vulnerabilities and same-user attacks against
Aporic state remain outside its guarantee. A receipt proves the recorded process
result, enforcement evidence, and declared artifact hashes—not test quality or a
broader real-world effect. Do not store secrets or raw conversation history in
Aporic records, and do not treat the profile as authorization to run arbitrary
hostile code.

## Secure capability boundary

The capability catalog stores bounded manifests and risk declarations as
untrusted data. Registration never loads plugin code, makes a provider
executable, grants credentials, opens a network path, or creates a generic tool
invocation surface. Tool annotations and manifest claims are not enforcement.

Prototype experiment hard gates require Aporic-direct evidence or an
observed/verified claim. A preference score cannot override a failed hard gate,
and final selection additionally requires a deliberation decision with no open
material issue. These are integrity rules for Aporic state, not host approval.

The Codex Security adapter is a local artifact importer, not a scanner runner.
It accepts bounded regular non-symlink JSON files, hashes the exact bytes read,
and records declared coverage and finding counts against a clean Git snapshot.
Partial coverage or zero findings never proves safety. Imported files remain
untrusted content, and Aporic does not pass tokens or credentials to a provider.

## Resource and recovery limits

v0.13 bounds hook input, direct workspace evidence, receipt artifact count and
bytes, and Git subprocess output. File digests are streamed rather than built
from whole-file allocations. These are process-availability controls, not
tenant isolation or authentication.

v0.14 adds sandbox-backend attestation to execution receipts and a fail-closed
Linux isolation profile. The timeout still provides the only runner-owned
resource-duration bound; deployments needing hostile multi-tenant execution
must add a stronger outer isolation and quota layer.

Restore validates and migrates a copy before atomically installing it at a new
destination and refuses existing targets. Retention deletes only regular,
non-symlink `aporic-backup-*.sqlite3` files after an explicit keep count. Stop
all processes using a database before manually replacing an active database.

## Reporting

Report suspected vulnerabilities privately to the repository maintainers. Do
not include secrets or exploit unrelated systems while preparing a report.
