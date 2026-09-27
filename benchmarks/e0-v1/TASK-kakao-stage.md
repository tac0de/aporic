# Direct subject quizzes with an incomplete learner profile

The Kakao bot's direct commands `/퀴즈 수학`, `/퀴즈 과학`,
`/퀴즈 국어`, and `/퀴즈 영어` must return a usable quiz card even
when a saved learner profile has no school stage. Use the middle stage as the
fallback only when no valid stage is available from the existing sources.
Preserve explicit elementary, middle, and high stage choices and the normal
answer/review flow.

Add a focused regression test at `tests/e0-stage-regression.test.ts`.
Keep product changes within `src/features/quiz/quizFlowService.ts`; the new
test is the only other allowed changed file. Do not
change dependencies, state schema, authentication, deployment, or production
configuration. Do not commit, push, deploy, or contact the network.

Stage 1 is investigation and a concise handoff only. Follow the supplied
two-session protocol for the handoff and Stage 2 implementation.
