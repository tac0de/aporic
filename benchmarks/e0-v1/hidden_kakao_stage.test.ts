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

test("explicit school stages survive the direct quiz fallback", async () => {
  const profiles = new MemoryLearnerProfileStore();
  const coach = createDisabledCoach({
    dependencies: { learnerProfileStore: profiles }
  });

  for (const stage of ["elementary", "middle", "high"] as const) {
    const userId = `e0-explicit-${stage}`;
    await profiles.set(
      `user:${userId}`,
      { schoolStage: stage, updatedAt: Date.now(), expiresAt: Date.now() + 120_000 },
      120
    );

    const response = await coach.respond({
      utterance: "/퀴즈 수학",
      conversationKey: `e0-explicit-room-${stage}`,
      userId
    });

    assert.ok(response.quizCard, `${stage}: direct quiz did not return a card`);
    assert.equal(response.quizCard.schoolStage, stage);
  }
});

test("a general profile still receives the middle-stage direct quiz fallback", async () => {
  const profiles = new MemoryLearnerProfileStore();
  const coach = createDisabledCoach({
    dependencies: { learnerProfileStore: profiles }
  });
  const userId = "e0-general-stage";
  await profiles.set(
    `user:${userId}`,
    { schoolStage: "general", updatedAt: Date.now(), expiresAt: Date.now() + 120_000 },
    120
  );

  const response = await coach.respond({
    utterance: "/퀴즈 과학",
    conversationKey: "e0-general-room",
    userId
  });

  assert.ok(response.quizCard);
  assert.equal(response.quizCard.schoolStage, "middle");
});
