---
title: "Give each git worktree its own cargo target folder"
author: DayZModdingTemplate
date: 2026-11-06
id: l-d603a560
scope: repo
status: active
src: agent
---

## Decided
Run `cargo test` with the default target folder inside the repo, so each worktree has its own. Tests that find fixtures through a path baked in at compile time (`env!("CARGO_MANIFEST_DIR")`) keep the path of whichever worktree built first.

## Tried and ruled out
Pointing `CARGO_TARGET_DIR` at one shared cache folder for every worktree. After the worktree that built it was removed, three image tests failed with "No such file or directory" although no code had changed. Deleting the folder fixed it at once.

## Watch out
The failure looks like a real regression in the code under test. Read the path in the panic message before looking at the code.
