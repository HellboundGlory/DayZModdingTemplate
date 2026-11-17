---
title: "Add shared names on the main branch before parallel jobs start"
author: DayZModdingTemplate
date: 2026-11-04
id: l-6fe2b57c
scope: repo
status: active
src: agent
---

## Decided
When two parallel jobs both need a new shared constant (a theme token, a setting key, an enum value), add it on the main branch first and mark that file read-only for both jobs. Two jobs each specified a 2 px focus edge under different names; one shared name now serves both.

## Tried and ruled out
Letting each job add its own rows. Both worktrees edit the same lines, and a name added in one worktree does not exist in the other, so its checks fall back to a default.

## Watch out
If a name must be listed in several places (code default, docs table, a self test table), update all of them in the same change, and refresh any line numbers that notes cite in the file you added constants to.
