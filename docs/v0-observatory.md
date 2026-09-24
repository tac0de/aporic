# V0 live observatory

The observatory is a read-only host integration. It serves one embedded,
dependency-free web interface and a literal JSON event surface from the
external Aporic state directory. It never writes a governed repository or any
ledger.

## Display model

The Joseon court vocabulary is presentation only:

| Display label | Literal source |
| --- | --- |
| 근정전 · 판단석 | latest open handoff objective, questions, and next action |
| 의정부 · 실행 | kernel reservations, effects, and verification |
| 승정원 · 연속성 | handoff days and session state |
| 호조 · 자원 | M1 model-control plans and process outcomes |
| 조보 | three separate append-only record streams |

The API never emits offices, ranks, royal commands, or implied authority.
Events from different ledgers are displayed separately because Aporic has no
single timestamp or causal order spanning those ledgers.

## Run locally

```console
aporicctl observe /absolute/state/connection.json
open http://127.0.0.1:4242
```

The browser receives an initial snapshot and then a server-sent event whenever
the underlying project observation or replayed ledger state changes.

## Bind beyond localhost

An external bind is explicit and requires a token:

```console
export APORIC_OBSERVATORY_TOKEN='replace-with-at-least-24-random-bytes'
aporicctl observe /absolute/state/connection.json 0.0.0.0:4242
```

Static presentation assets reveal no project state. `/api/snapshot` and
`/api/events` require the bearer token. V0 provides HTTP only; expose it beyond
a trusted local network only behind a separately configured TLS reverse proxy.
