---
title: "Chat commands must fit vanilla's chat box, which cuts text by width"
author: DayZModdingTemplate
date: 2026-11-02
id: l-c277e47f
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Decided
Give chat commands a short prefix. Vanilla's chat box stops taking text once it is visually full, so long command words get cut.

## Tried and ruled out
- Reading a "no such setting" reply as a lookup bug. The settings code was right: the line was cut before it was sent, so a long key arrived chopped and a key with a value lost the value.
- Lifting the limit from script. `EditBoxWidget` only has `GetText` and `SetText`.

## Watch out
The limit is width, not a character count, about 49 characters at 1920x1080. A cut line gives a misleading reply, not an error, so any new command or setting name must fit together with its value.

In the code: vanilla `gui/layouts/day_z_chat_input.layout` has `"limit visible" 1`.
