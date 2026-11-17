---
title: "Play UI sounds with autodestroy and do not hold them"
author: DayZModdingTemplate
date: 2026-11-03
id: l-3328144c
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Decided
Call `SetAutodestroy(true)` on a sound right after `SEffectManager.PlaySound(...)` and keep no reference to it.

## Tried and ruled out
Keeping the last sound per role in a map so it is not freed early. The effect manager already registers every sound it creates and holds a strong reference until the sound is destroyed. Without autodestroy, every click adds one effect that is never freed.

## Watch out
Any one-shot effect played through the effect manager needs the same call.

In the code: vanilla `scripts/3_Game/EffectManager.c` (register keeps it in `m_EffectsMap`), `scripts/3_Game/Effects/EffectSound.c:607`.
