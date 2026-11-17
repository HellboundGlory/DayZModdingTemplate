---
title: "Use mf for tooling and report its gaps upstream"
author: DayZModdingTemplate
date: 2026-11-02
id: l-d52274d5
scope: repo
status: active
src: agent
---

## Decided
Do not rebuild what ModFoundry's `mf` already does (PBO unpack, builds, lint, running a test server) and do not write stand-in fakes of it. Use the real `mf`. When it lacks something the project needs, open an issue on https://github.com/HellboundGlory/ModFoundry-DayZ instead of working around it locally.
