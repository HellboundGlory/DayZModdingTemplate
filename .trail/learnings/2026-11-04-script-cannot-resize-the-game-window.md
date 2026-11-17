---
title: "A script cannot resize the game window"
author: DayZModdingTemplate
date: 2026-11-04
id: l-25bd9f17
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Decided
To photograph or test a UI at several sizes, start the client at each size with `mf run-client --window <WxH>`. Do not try to change the window from script.

## Tried and ruled out
Changing the resolution through the video options option (`AT_OPTIONS_RESOLUTION`, `3_Game/gameplay.c:1242`) from a test rig. That writes the player's own saved video settings. A manual change in the menu is fine and needs no restart (see the video options learning).

## Watch out
No vanilla script calls `MakeScreenshot(string name)` (`1_Core/proto/proto.c:142`). In a retail client it seemed to save nothing, but the check looked in `ScreenShots/` while `proto.c:139` names `$profile:ScreenShotes/`, so that result is doubtful too; whether a shot includes the UI is unknown.
A script-made static camera turned off with `SetActive(false)` and then deleted did not hand the view back to the player in a retail client (2026-11-04): the world went black, Esc stopped reaching the menu and the game stopped responding. Prove how the player's view comes back before using a script camera on the client. Vanilla can only tell you whether left Ctrl is down (`DayZGame.IsLeftCtrlDown()`) and has no Shift reader. A keys file in 5_Mission cannot be read from 3_Game code.
