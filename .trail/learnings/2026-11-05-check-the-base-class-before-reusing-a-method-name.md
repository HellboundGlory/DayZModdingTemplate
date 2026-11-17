---
title: "Check the base class for a method name before reusing it with other parameters in a subclass"
author: DayZModdingTemplate
date: 2026-11-05
id: l-72965472
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Decided
Before adding a method to a subclass, read the base class for the same name. A base with a protected `Bind()` and no parameters looked like it would block a subclass `Bind(controller, field)`, so the subclass method was named `BindField` instead. This was read in the base class, not proven by a build; a client compile is still needed to prove the clash. Overloads in one class do exist (Community Framework's `CF_Trace` has `Add(bool)`, `Add(int)`, `Add(float)` side by side), so only a subclass reusing a base name is in doubt.

## Watch out
Vanilla `Math` has float-only `Min`, `Max` and `Clamp` (`EnMath.c:540, 553, 566`), so int clamping has to be written by hand.
