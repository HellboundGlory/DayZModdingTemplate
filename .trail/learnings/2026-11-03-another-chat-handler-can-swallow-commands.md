---
title: "Another mod's chat handler can swallow your chat commands"
author: DayZModdingTemplate
date: 2026-11-03
id: l-ed405469
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30. The failure was seen in game; the fix below was read from source and not yet proven in a boot.

## Decided
Do not rely on the chat menu's `OnChange` for commands. Another chat mod can replace the menu so that it sends every line itself and never calls the code your patch added, whatever the `-mod` order. Instead set your own handler on the chat edit box when chat opens: take a registered command line, clear it, and forward every event to the menu.

## Tried and ruled out
- Changing the `-mod` order: no effect (tried both ways, commands went out as plain chat).
- A separate add-on mod loaded after the chat mod: one more install, and forgetting it brings the bug back.
- A keybind command box: leaves the typed command in chat.

## Watch out
`Widget` has no `GetHandler`, so two mods that both set a handler on the same box fight; the last to attach wins. Check that normal chat is sent once, not twice.

In the code: vanilla `5_Mission` `ChatInputMenu.c` (`OnChange`); `Widget.SetHandler` at `1_Core/proto/EnWidgets.c:172`.
