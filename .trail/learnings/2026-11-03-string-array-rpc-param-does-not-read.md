---
title: "A string array in one RPC param does not read back"
author: DayZModdingTemplate
date: 2026-11-03
id: l-e635b333
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30. Measured in a server boot on 2026-11-03.

## Decided
Send a count followed by one string param per piece, not a list of strings packed into one param.

## Tried and ruled out
- `Param1<array<string>>`: the receiving `ctx.Read` failed every time. Nothing in vanilla or CF sends one either.
- Calling an RPC handler from a 3_Game check through `CallFunctionParams`: the handler did not seem to run. Put the check in 4_World and call the module directly.

## Watch out
In a check, a param written to a `ScriptReadWriteContext` must stay referenced, by its own type, until it is read. A temporary `new Param1(...)` passed to `Write` fails, and so does writing through an `array<ref Param>` base-typed reference. The context seems to keep the object, not a copy.
