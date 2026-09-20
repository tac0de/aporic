# Security

## Project status

Aporic `0.0.x` is an experimental governance kernel, not a hardened security boundary. Do not rely on it as the sole control for untrusted code execution, secrets, production deployment, or access control.

## Current trust boundary

- Event actor identity and provenance are caller-declared and are not cryptographically authenticated.
- File locking coordinates cooperating processes under the same operating-system user; it does not prevent same-user tampering.
- The Codex adapter can fail closed when its configured store is missing, invalid, or busy, but it cannot guarantee behavior when the hook process itself is bypassed, killed, or never invoked.
- The initial preventive configuration covers a named tool such as `apply_patch`; shell commands, hosted tools, and other mutation paths remain outside that gate unless separately integrated.
- Plan authorization proves that a matching record exists. It does not prove that the plan is good or that a proposed patch semantically follows it.
- Authorization is checked before a tool runs; it is not atomically bound to the later filesystem effect. State can change between the check and the effect.
- Successful appends request data synchronization with `sync_data`, but Aporic does not claim power-loss durability: parent-directory metadata is not synchronized and there is no repair, migration, or snapshot mechanism.
- `SessionStart` projects recorded text as untrusted context. It does not provide prompt-injection immunity or make event contents authoritative instructions.
- Replay rejects malformed sequences and unsupported schema versions, but it does not rerun the current authorization policy over historical events.

## Reporting a vulnerability

Do not open a public issue for a vulnerability that would expose sensitive details. Use GitHub's private vulnerability reporting for `tac0de/aporic` when available. Include affected revision, reproducer, expected boundary, observed behavior, and potential impact.

Until a response policy is published, no response-time or remediation-time guarantee is made.
