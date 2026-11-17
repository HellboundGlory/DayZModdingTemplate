---
title: "Dabs options tabs and inputs file groups, read from source"
author: DayZModdingTemplate
date: 2026-11-02
id: l-479bf4b6
scope: repo
status: active
src: agent
---

Read from source, not run in game. Check anything here in a client before relying on it.

## Tried and ruled out
One shared options tab for several mods is not something Dabs offers directly. `OptionsMenu.c` adds one tab per registered `ProfileSettings` class and skips a class with no rows, and a row attribute binds to the last registered class (`RegisterProfileSettingBase.GetCurrentProfileSetting`). A sibling mod would have to patch the first mod's settings class to add rows to its tab.

## Watch out
- Dabs has four row types: Color, Multistate, Slider, Text. There is no plain bool row.
- An inputs file's `<sorting name>` is shared across separate PBOs in another large mod, which suggests groups with the same name merge in the Controls menu. Unproven until a client try.
- Vanilla's default for the `/` key cannot be seen in the reference scripts.

In the code: Dabs `OptionsMenu.c`, `RegisterProfileSettingBase`.
