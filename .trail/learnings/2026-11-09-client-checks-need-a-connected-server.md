---
title: "Client checks run only on a client connected to a server"
author: DayZModdingTemplate
date: 2026-11-09
id: l-96f33419
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Decided
Client checks start at mission start, so a lone client at the main menu only proves the client scripts compile. Start a server with the test mod and connect a client to it, then read the `DONE side=client pass= fail=` line in the client's script log, not mf's verdict.

## Tried and ruled out
A lone client with `--set selftest` and no connect: every module compiled, no check ran, and mf reported the expect-line missing.

## Watch out
- An unfocused game window sits on the pause menu. Loading took about 4 minutes, the checks waited 60 seconds for focus and started anyway, so mf's 300 second window closed before DONE. The checks still passed and the line appeared a minute later.
- `mf run-client`'s timeout leaves the client open, and the next run is refused ("a DayZ client is already running") until it is stopped by exact process id, including its pressure-vessel wrappers.
