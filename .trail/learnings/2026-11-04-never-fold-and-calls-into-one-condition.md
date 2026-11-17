---
title: "Do not fold method calls into one && condition or inline a fresh view"
author: DayZModdingTemplate
date: 2026-11-04
id: l-a4331252
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Decided
In Enforce Script keep two-step conditions split. Do not join a method call into an `&&` or `||`, and do not pass a freshly made view straight into `Add` without a local holding it first. The `&&` trap was seen in boots (2026-10-31 and 2026-11-02); the hold-the-view-in-a-local one was read in source and not yet run in game. A simplify pass proposed 21 edits and nine had to be reverted for this.

## Tried and ruled out
Letting automated simplify passes fold nested ifs and inline returned views. Lint, build and tests all passed, but none of them runs client-only code.

## Watch out
Tell simplify tools about these traps up front. A server boot does not compile `#ifndef NO_GUI` code, so a client-only break passes every server-side check.
