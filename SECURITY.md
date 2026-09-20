# Security

## Project status

Aporic `0.2.x` is an experimental governance kernel, not a hardened security boundary. Do not rely on it as the sole control for untrusted code execution, secrets, production deployment, or access control.

## Current trust boundary

- Event actor identity and provenance are caller-declared and are not cryptographically authenticated.
- File locking coordinates cooperating processes under the same operating-system user; it does not prevent same-user tampering.
- The Codex adapter can fail closed when its configured store is missing, invalid, or busy, but it cannot guarantee behavior when the hook process itself is bypassed, killed, or never invoked.
- The initial preventive configuration covers a named tool such as `apply_patch`; shell commands, hosted tools, and other mutation paths remain outside that gate unless separately integrated.
- Plan authorization proves that a matching record exists. It does not prove that the plan is good or that a proposed patch semantically follows it.
- Authorization is checked before a tool runs; it is not atomically bound to the later filesystem effect. State can change between the check and the effect.
- Bounded-grant evaluation and consumption are atomic with respect to cooperating Aporic writers, but consumption means only that preflight admission was issued. It does not attest tool success or effects.
- Exact tool input for a bounded grant is stored in the append-only log. Do not place credentials, tokens, private keys, or other secrets in granted input.
- Successful appends request data synchronization with `sync_data`, but Aporic does not claim power-loss durability: parent-directory metadata is not synchronized and there is no repair or online snapshot mechanism. The explicit migration command creates a validated offline snapshot copy; it is not recovery.
- `SessionStart` projects recorded text as untrusted context. Its execution status is a sampled observation at one recorded revision, not permission, and it can become stale before a later tool call. It does not provide prompt-injection immunity or make event contents authoritative instructions.
- The 6,000-byte projection limit bounds injected UTF-8 bytes; it does not promise a token count, model-cost reduction, or retention of every detailed record. For observed state, critical gate status and authority counts remain as per-tool entries or explicit omitted-tool aggregates, blocker kinds are retained, and detail omission is explicit. Unavailable state has no trusted status to aggregate and reports only unknown retained entries plus protected and omitted counts.
- FNV identities in diagnostics and omission receipts are non-cryptographic lookup aids. The authorization boundary compares canonical JSON values exactly and never trusts these identities.
- Replay rejects malformed sequences and unsupported schema versions, but it does not rerun the current authorization policy over historical events.

## Reporting a vulnerability

Do not open a public issue for a vulnerability that would expose sensitive details. Use GitHub's private vulnerability reporting for `tac0de/aporic` when available. Include affected revision, reproducer, expected boundary, observed behavior, and potential impact.

Until a response policy is published, no response-time or remediation-time guarantee is made.
