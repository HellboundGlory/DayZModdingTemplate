---
title: "Enforce has no file seek, so isolate logs per boot"
author: DayZModdingTemplate
date: 2026-11-03
id: l-f903b43f
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30. Measured on 2026-11-03.

## Decided
Checks that confirm a line was logged by reading a log file before and after read the whole file each time. A mod's daily log holds every boot of the day (1.2 MB), so each read cost about 4 seconds and the suite got slower with every boot. Start the test boot with its own empty profiles folder and empty the mod's logs folder first, so a read sees only this boot's lines. Never touch the play profile.

## Tried and ruled out
Reading only the new end of the file. Enforce has no seek (`EnSystem.c` only has `OpenFile`, `FGets`, `ReadFile`), so any read walks the whole file.

## Watch out
Time a boot from the script log's own line timestamps before guessing. Here 59 gaps of a second or more took 275 of 278 seconds, and the other ~2,690 checks took about 3. `TickCount` is about 10,000 per millisecond on Diag and an int, so time one check class at a time, never the whole run.
