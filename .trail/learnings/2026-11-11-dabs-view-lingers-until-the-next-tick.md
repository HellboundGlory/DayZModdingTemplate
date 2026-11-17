---
title: "A destroyed Dabs view is still loaded until the next GUI tick"
author: DayZModdingTemplate
date: 2026-11-11
id: l-535a0825
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Watch out
Dabs `ScriptView.Delete()` only queues the real delete (`CallLater(_delete)`, `ScriptView.c:210`). Hiding the root and calling it leaves the root loaded, so a check that destroys a view and then asserts on `IsLoaded()` or the root sees the old state. In game the next frame flushes the delete, so the cost is one frame. In a check, close through the owner or assert after the GUI queue has run.
