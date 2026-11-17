---
title: "Two Inkscape runs at once abort during the version check"
author: DayZModdingTemplate
date: 2026-11-05
id: l-81c58bab
scope: repo
status: active
src: agent
---

## Watch out
Starting Inkscape twice at the same time made `inkscape --version` abort with SIGABRT (3 of 3 pairs). A guard that checks for the program then reports it missing or failed, which looks like an install problem. In an image bake pipeline, run one Inkscape at a time, and run gated tests that call it one at a time too (3 of 3 parallel test runs failed, 3 of 3 serial runs passed).
