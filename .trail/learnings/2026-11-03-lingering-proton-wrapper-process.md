---
title: "A DayZ Diag client can be hung when only its Proton wrapper is left"
author: DayZModdingTemplate
date: 2026-11-03
id: l-66fa20f3
scope: repo
status: active
src: agent
---

## Decided
Treat a Diag process tree as hung when the game executable is gone but the Proton and Steam runtime wrappers that launched it are still running. Stop them by exact process id, because a name pattern can hit a game you are playing.

## Watch out
The wrapper's command line still names `DayZDiag_x64.exe`, so a `ps` listing looks like a live client. Look for the exe's own process first. The tree is srt-bwrap, pv-adverb, `proton run`, `steam.exe`, then the game exe; the hung case has no row for the exe itself. A wrapper left from an earlier run survives later runs and shows up an hour later as "a DayZ client is already running".
