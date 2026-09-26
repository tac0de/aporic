# C04 empty CLI output: follow-up inspection

The original treatment run's `python3 aporic_read.py recall` command completed
with exit code 0, but its Codex CLI `aggregated_output` was empty. The frozen
exact-answer score remains unchanged. That trace alone does not show how the
model obtained rule `FE-R3`.

On 2026-09-27, a direct rerun of the helper against the preserved isolated
C04 database returned 2,597 bytes of valid JSON (`sha256
788862238b9a8fbf604de0bfcd736795a3a25bd4e5353f5c9b0faea6366becbf`),
including the FE-R3 decision. A separate Inspector reported that a fresh
`codex exec` session in the same workspace also captured the full output and
identified FE-R3. The original batch's other treatment recall traces had
nonempty captured output. The CLI stderr's state DB fallback warning was
present in the successful reported reproduction as well.

The evidence favors an isolated CLI command-output capture anomaly over an
Aporic MCP retrieval failure. Its root cause cannot be established from the
retained trace. This case supports exact answer correctness, but not a claim
that the model visibly used the retrieved record. Future runs should retain a
helper-owned transcript of actual MCP replies and compare it with CLI event
capture; mismatches remain explicit instrumentation anomalies.
