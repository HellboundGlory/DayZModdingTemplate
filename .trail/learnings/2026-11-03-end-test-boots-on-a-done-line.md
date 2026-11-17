---
title: "End test boots on a DONE line with mf's expect-line"
author: DayZModdingTemplate
date: 2026-11-03
id: l-5bf60620
scope: repo
status: active
src: agent
---

Measured with mf 1.5.0 on 2026-11-03; recheck on newer mf.

## Decided
`mf run-diag --timeout` is only a launch window: the server runs until it closes, even after the test mod has finished. mf stops the run as soon as every `--expect-line` has appeared (7 ms after the line). Have the test mod print a final DONE line and pass it as the expect-line; the timeout becomes an upper limit. One boot went from 421 s to 67 s.

## Tried and ruled out
Matching `DONE ... fail=0`: a boot with a failed check never matches and sits out the whole window. Match any DONE line and let the script fail the run on FAIL lines and counts.

## Watch out
mf fails the verdict on any mod script error. CF prints one JSON error every boot (`JM/CF/Scripts/3_Game/communityframework/game/dayzgame.c:46`), so pass it to `--allow-error`. A new harmless framework error will fail good boots the same way; read mf's verdict lines before suspecting your own code.
