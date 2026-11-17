---
title: "EnScript.GetClassVar returns 0 even when the read worked"
author: DayZModdingTemplate
date: 2026-11-04
id: l-27844aad
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.164289. Seen in a client log, 2026-11-04.

## Decided
Do not trust the return value of `EnScript.GetClassVar` (or `SetClassVar`). Seed a local with a known value, read into it, and judge by the value.

## Tried and ruled out
Treating 0 as failure. The read returned early every time, so bound controls opened on the wrong value and the "could not write" warning could never fire.

## Watch out
Not proven that a 0 answer always fills the out value.
