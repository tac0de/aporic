# Codex subagent model selection

Model selection belongs to the host. Aporic's route recommendation is advisory;
it neither selects a host model nor attests to the model that executed a turn.

## Why a child can use a different model

The [official OpenAI subagent documentation](https://learn.chatgpt.com/docs/agent-configuration/subagents)
describes resolution from an explicit spawn value, then the corresponding
`[agents]` default, then the parent setting. A selected custom agent file can
override model and effort afterward. If a spawn or default selects a model but
neither supplies effort, the model's default effort applies.

The local model-only configuration check on 2026-10-03 found:

```toml
model = "gpt-6.1-sol"
model_reasoning_effort = "medium"

[agents]
default_subagent_model = "gpt-5.6-terra"
default_subagent_reasoning_effort = "low"
```

Those child defaults explain the Terra/low selection when a spawn omits explicit
values. This is a configuration explanation, not evidence that Codex assessed
the task and chose Terra as the most suitable model. The configuration was read,
not changed. Recheck local settings before applying this historical observation
to another session.

## Dispatch intentionally

For ordinary Aporic implementation, select `gpt-6.1-sol` explicitly. Choose
effort for the task; the 2026-10-03 implementation and diagnosis workers requested
`high`. A lighter selection needs a bounded mechanical task and an explicit
reason. Default settings alone provide no comparative evidence that Terra/low
is suitable for general implementation or independent review.

With the current Codex collaboration tool, an override requires `fork_turns` to
be `"none"` or a bounded turn count; a full-history fork does not accept model or
effort overrides. Send a concise work packet containing scope, edit permission,
expected result, and acceptance checks, and set actual tool arguments:

```json
{
  "task_name": "bounded_worker",
  "fork_turns": "none",
  "model": "gpt-6.1-sol",
  "reasoning_effort": "high",
  "message": "Bounded task contract with the context needed to complete it."
}
```

Naming a model only in the message does not set a tool argument. If explicit
selection fails, report the failure; do not silently substitute another model.

## Keep evidence separate

Record configured defaults, requested spawn values, observed host metadata, and
execution attestation separately. Model/effort fields in local thread metadata
can confirm the host's stored selection. On 2026-10-03 they showed Terra/low for
the earlier inventory child and Sol/high for the new implementation and diagnosis
children. They do not independently attest to the backend model that executed
each response. Report executed model as unknown unless direct execution evidence
is available. Inspect only necessary model metadata, never raw conversation or
secrets, and keep Aporic delegation reports honest about that distinction.
