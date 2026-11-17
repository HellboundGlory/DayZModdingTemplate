---
title: "mf lint warns about a foreach straight over a method's return value, but still exits clean"
author: DayZModdingTemplate
date: 2026-11-17
id: l-4826a517
scope: repo
status: active
src: agent
---

ModFoundry 2.2.0.

## Decided
Holding a call's return in a local before a `foreach` is still the rule; the in-game crash it guards against is unchanged. Since ModFoundry 2.2.0, `mf lint` flags `foreach (int i : Items())` with `MF-W3005 ... hold the result in a local first`, so `docs/enforce-script.md` lists it under what the lint catches.

## Watch out
`MF-W3005` is a warning, not an error, so the lint still exits 0. Read the findings, not the exit code.
