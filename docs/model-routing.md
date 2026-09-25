# Advisory frontier-model routing

The router uses typed task properties rather than prompt wording. Its output is
a recommendation, never authorization, dispatch, or evidence.

| Route | Use |
| --- | --- |
| `gpt-5.6-terra` | Bounded, well-specified, low-to-medium consequence work |
| `gpt-6-sol` | Default complex implementation and agentic tool work |
| `gpt-6-astra` | Frontier complexity, critical consequence, or high-consequence ambiguity |

When independent review is requested, Astra reviews Terra/Sol work and Sol
reviews Astra work. This reduces identical-pass coupling, but a second model is
still a model assessment and therefore never counts as direct evidence.

The model IDs and intended capability tiers were checked against the official
OpenAI model documentation on 2026-09-25:

- <https://developers.openai.com/api/docs/models>
- <https://developers.openai.com/api/docs/models/gpt-5.6-terra>

Model roles belong to this evolving hub policy and stay outside the stable
behavioral kernel.
