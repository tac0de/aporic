# Aporic human-facing lexicon

## Day

A **day** means one chat session in human-facing conversation.

This is only a continuity metaphor. It does not mean a calendar day, wall-clock
duration, identity, deadline, expiry, authority, or kernel state. Starting a new
day does not transfer session-bound grants or unconsumed execution authority.

## Handoff capsule

A **handoff capsule** is a bounded continuity record for a successor day. It
contains the objective, constraints, accepted decisions, completed checks, open
questions, and next executable action, sealed together with the observed
project binding, Git HEAD, and dirty state.

The capsule is evidence supplied by the host, not authority or an assertion
that its natural-language contents are true. A successor explicitly names the
exact capsule digest it inherits. The workspace handshake must still match.
