---
title: "Keep comments out of published mod scripts"
author: DayZModdingTemplate
date: 2026-11-02
id: l-a54a7bc9
scope: repo
status: active
src: agent
---

## Decided
Published mod code carries little to no comments, only a short line where the reason is not obvious. Tests may have more. Usage notes belong in docs, not in code comments.

## Watch out
Generated code from coding agents tends to be comment-heavy, so check diffs for it before merging. `mf build` strips comments from the packed scripts (since mf 1.4.0), so the comments that stay in source do not ship.
