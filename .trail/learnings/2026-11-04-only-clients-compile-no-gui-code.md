---
title: "Only a client compiles #ifndef NO_GUI code"
author: DayZModdingTemplate
date: 2026-11-04
id: l-5d26e2ee
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.164289. First found 2026-11-04.

## Decided
A server boot never compiles code inside `#ifndef NO_GUI`, so a break there shows only in a client build. The first client build of one project found four breaks that lint, unit tests and the server self test all passed. Since mf 2.1.0, `mf lint` with CF and Dabs as dependencies compiles that code like the Diag client (findings tagged `[diag-client]`). It caught three of those four, but it is still a model, so run a lone client to the main menu before relying on client code.

## Tried and ruled out
Relying on the server self test boot.

## Watch out
- The engine stops at the first failing module, so fix and rerun until the Mission module loads. "Bad type 'array'" and "Can't find class Param1" are knock-on errors; the real one is the line after "Can't compile".
- `TextWidget` has no text getter in 1.30. Call `Update()` then `GetTextSize`.
- A `UAIDWrapper` cannot be held by `ref` (its destructor is private). The lint check still passes this.
- A client left at the main menu blocks `mf` from starting a server ("a DayZ client is already running"). Stop it by exact process id.
