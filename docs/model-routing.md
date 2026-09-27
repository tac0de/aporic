# Advisory frontier-model routing

The router uses typed task properties rather than prompt wording. Its output is
a recommendation, never authorization, dispatch, or evidence.

The current route table is a provisional static policy. Aporic never invokes a
model or the OpenAI API. The offline evaluator can grade structured trials, but only a
non-empty batch whose every result is host-attested is eligible to inform a
future policy revision. Deterministic simulator output and operator/model
reported identities are useful for testing and diagnosis, but cannot change the
route table.

| Route | Use |
| --- | --- |
| `gpt-6-luna` | Bounded, well-specified, low-to-medium consequence work |
| `gpt-6-sol` | Default complex implementation and agentic tool work |
| `gpt-6-astra` | Frontier complexity, critical consequence, or high-consequence ambiguity |

When independent review is requested, Astra reviews Luna/Sol work and Sol
reviews Astra work. This reduces identical-pass coupling, but a second model is
still a model assessment and therefore never counts as direct evidence.

The model IDs and intended capability tiers were checked against the official
OpenAI model documentation on 2026-09-27:

- <https://developers.openai.com/api/docs/models>
- <https://developers.openai.com/api/docs/models/gpt-6-luna>

Model selection belongs to this evolving hub policy and stays outside the stable
behavioral kernel.

The v0.6 Codex hook may receive the active model slug from the host. Aporic
labels that value as a non-attested host observation and never uses it to grant
authority, establish evidence, or mutate this route table.
