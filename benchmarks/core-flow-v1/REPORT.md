# Short core workflow: operational evidence

Date: 2026-10-03 (Asia/Seoul). This is an implementation/continuity trial, not the frozen paired instrument pilot in docs/core-ops-cost.md.

## Actual follow-on changes

- Probe: updated the actual `benchmarks/core-ops-v1/probe.py` from legacy open/record/close to begin/finish, continuation selection, request-byte reporting and raw capture. Session one implemented the path and handed off validation; session two used resume, recovered the note, executed three repetitions, and completed.
- Documentation: updated README, docs/core-flow.md and the Codex bridge template, then handed off API/bound/evidence validation. Session two used resume, recovered the exact next action and note, checked documented behavior against APIs/tests, and completed.

Each used two durable Aporic sessions and four independent direct stdio connections (start, handoff, continue, complete), with a separate local trial DB and the actual repository workspace. One host conversation retained context; these were not independent fresh-model trials. The trial DB isolates selection from this repository's unrelated active tasks. Parent-task independent Inspector reviewed the resulting artifacts; the bounded follow-ons did not spawn additional agents. Raw [probe session trace](probe-session-trace.json) and [documentation session trace](docs-session-trace.json) include full returned continuity snapshots and connection initialization. Durable records contain reported observations, not verified effect claims.

| Change | Durable sessions | Lifecycle calls | Request bytes | Response bytes | Tool round trips total | Elapsed wall time |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| probe | 2 | 5 | 2,870 B | 7,396 B | 90.20 ms | 54.60 s |
| docs | 2 | 5 | 3,175 B | 7,804 B | 81.66 ms | 534.32 s |

Each sequence is begin → finish(handoff) → resume → begin → finish(completed). Continuation selected the preceding handoff and returned its durable observation in both changes. Raw byte counts include JSON-RPC response framing. Elapsed wall time includes concurrent parent-task activity and is not isolated task cost. Authoring minutes, host/model tokens, owner correction/restatement time and fresh-model rediscovery were not measured. No user clarification was requested during these follow-ons; that alone cannot establish reduced owner effort.

The trial binary was an instrumented debug build, observed package version 0.28.0, source SHA-256 `9cecad250a3462ea2fe1294375d0804c43c068cba56da2c39bdd95182ec0c1f6`. Its executable digest is in each trace. Subsequent source refinements (avoiding a duplicate recall and aligning bounded implementation routing with gpt-6.1-sol) are not retroactively credited to these captures.

## Direct release probe

The [release stdout](release-probe-result.txt) and [raw release replies](release-probe-raw.json) retain three local repetitions with fresh DBs and one initialized process per repetition. The five-call median total was 18.06 ms, with 1,826 request bytes and 6,398 response bytes per sequence. First begin median was 4.52 ms and 2,055 response bytes. This measures direct local stdio after initialization; it excludes model work, host routing, authoring, startup and practical review/evidence calls. There is no baseline arm, so it is not evidence of a net task-time improvement. Byte sizes are not provider tokens. The earlier [debug capture](probe-result.txt) and [raw debug replies](probe-raw.json) are retained with their distinct binary digest.

The [direct route check](route-check.json) separately observed bounded implementation recommending `gpt-6.1-sol` through the full-profile MCP server. It reports selection advice and does not attest the host model.

## Mechanical acceptance and limits

Legacy open/record/close tests pass unchanged. New tests cover wrapper and legacy event persistence, exact retry and conflict, invalid assessment/note/completion gate rollback, unfinished-session resume, escaped context response bounds, and actual raw stdio begin/finish across restart. Injected SQLite append-failure coverage exercises rollback after opening or after recording/closing. No schema migration was added; typed effect and material-unknown gates remain in place.

Clippy and the workspace test suite were observed passing locally; affected tests are rerun after final refinements, and the pushed commit is checked by Linux/macOS/Windows CI. CI results belong to the exact run/commit and are not inferred from local success.

The release executable passes source/package/HEAD checkout validation and direct MCP initialization. Codex's current host MCP connection does not expose the new begin/finish tools yet. No host reconnect tool is available and native Codex UI automation is blocked by the tool. Replacing the release file therefore does not prove this conversation reconnected; host reconnect and a fresh begin/runtime observation remain the explicit operational handoff.
