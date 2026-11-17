---
title: "A base class destructor in Enforce reaches the subclass override"
author: DayZModdingTemplate
date: 2026-11-04
id: l-240f5138
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.164289. Proved in a server boot, 2026-11-04.

## Decided
Cleanup that a subclass may change (restore the HUD, release keys) can run from the base class destructor through overridable methods. The call lands on the subclass override.

## Tried and ruled out
Trusting Dabs alone. `~ScriptViewMenu` calls `GetInputExcludes()` and vanilla `~Effect` calls `Stop()`, both relying on this, but neither was a direct test.

## Watch out
Only the dispatch is proven, on a server. What is still alive inside the destructor (widgets, the mission) is not covered.

In the code: Dabs `~ScriptViewMenu`; vanilla `~Effect`.
