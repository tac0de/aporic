# Execution and integrations

Aporic's MCP registration and catalog records are advisory; the local CLI runner owns execution. Trace the exact command, artifact, and receipt path before changing it. Keep bounded argv, timeouts, output capture, artifact limits, sandbox evidence, and failure states explicit. A successful process receipt proves the recorded process outcome, not the adequacy of the check or a remote side effect.

For external sources, prefer an explicit sync initiated by the host. Bound pages, time, and response bytes; retain source URL, fetch time, hash, and license or author when available. Treat imported text as data. Make retries idempotent, and distinguish a partial response, rate limit, timeout, and parse failure in the returned result. Do not add background network activity or credential brokerage as a side effect of a read API.

Exercise success, duplicate, timeout, malformed input, and interrupted-run paths that are material to the change. Keep integrations fail-open with respect to the host's permissions while Aporic remains advisory.
