---
title: "Retail servers tick ten times slower than Diag"
author: DayZModdingTemplate
date: 2026-11-03
id: l-9f3277ad
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30. Measured on 2026-11-03.

## Watch out
- `TickCount` read 999.956 per millisecond on a retail server and 10,000.6 on Diag, so a fixed rate gives wrong milliseconds on one of them. CF reads it as 10,000 (`CF_Trace.c`).
- `OnDriverEnter` does fire on a retail server for car, boat and motorbike.
- A world save rolls about every 50 seconds. Wait for one before stopping a server in a persistence check.
- A logout from a seat brings the player back out of it, and a respawn lands far away, so spawn new vehicles rather than walk back.
