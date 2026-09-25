# Reliability and performance

Start with the observed workload and a baseline: latency, memory or database size, query plan, failure rate, or recovery time as appropriate. Optimize the limiting path and remeasure the same workload. Keep bounded context and output behavior intact; a faster result that drops required evidence is a regression.

Inspect cancellation, restart, duplicate delivery, disk or network failure, and partial-write behavior for stateful changes. Preserve atomic transactions, recovery paths, doctor audits, and backup/restore compatibility. Logs and traces should identify the failing operation without storing raw prompts, secrets, or unnecessary payloads.

Use a small, relevant verification set first, then expand to the full suite for cross-cutting changes. Report the measured environment and limits of the measurement. A local benchmark or green CI run does not establish production capacity or availability.
