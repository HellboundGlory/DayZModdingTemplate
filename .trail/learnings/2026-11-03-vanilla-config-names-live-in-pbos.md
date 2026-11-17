---
title: "Check vanilla config names in the game's own PBOs"
author: DayZModdingTemplate
date: 2026-11-03
id: l-bb6d03ff
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Decided
A vanilla config class such as a sound set is confirmed by unpacking the game's PBO with mf and reading the derapified config. An unpacked scripts folder has no config, so grepping it finds nothing and proves nothing.

## Watch out
A plain byte grep of the PBO shows the name is present, not that it is a class definition. Read the derapified line `class <name>: <parent>`.

In the code: `mf pbo-unpack <DayZ>/Addons/sounds_hpp.pbo --out <scratch>`, then `mf derapify <scratch>/DZ/sounds/hpp/config.bin`. Four UI sound sets confirmed this way, children of `baseCharacter_SoundSet`: `Fire_Mode_Switch_Simple_Click_SoundSet`, `digitalLock_beep_Access_SoundSet`, `digitalLock_mechanism_Unlock_SoundSet`, `digitalLock_beep_Error_SoundSet`.
