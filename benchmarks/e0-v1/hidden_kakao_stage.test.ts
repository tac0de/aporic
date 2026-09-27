import assert from "node:assert/strict";
import test from "node:test";

import { createDisabledCoach } from "./support/questionTestSupport";
import { MemoryLearnerProfileStore } from "../src/state/profile/learnerProfileStore";

test("a learner without a school stage can start every direct subject quiz", async () => {
  const profiles = new MemoryLearnerProfileStore();
  const coach = createDisabledCoach({
    dependencies: { learnerProfileStore: profiles }
  });

  for (const [index, subject] of ["수학", "과학", "국어", "영어"].entries()) {
    const userId = `e0-stage-absent-${index}`;
    await profiles.set(
      `user:${userId}`,
      { updatedAt: Date.now(), expiresAt: Date.now() + 120_000 },
      120
    );

    const response = await coach.respond({
      utterance: `/퀴즈 ${subject}`,
      conversationKey: `e0-room-${index}`,
      userId
    });

    assert.ok(response.quizCard, `${subject}: direct quiz did not return a card`);
    assert.equal(response.quizCard.schoolStage, "middle");
    assert.doesNotMatch(response.text, /퀴즈를 바로 준비하지 못했어요/u);
  }
});
