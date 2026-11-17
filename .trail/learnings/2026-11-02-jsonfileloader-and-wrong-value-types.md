---
title: "JsonFileLoader and wrong value types in settings files"
author: DayZModdingTemplate
date: 2026-11-02
id: l-db36b127
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30. Measured in a server boot on 2026-11-02.

## Decided
If one wrong type should cost one value and not the whole file, check the file text before the typed load.

## Watch out
- A string on a bool or an int makes the reader fail, so the whole file goes to defaults, and features the admin switched off come back on.
- `null` on a bool gives no error and reads false, whatever the default.
- A number on a bool gives no error; 2 reads true.
- A reading that equals the default proves nothing on its own. Check the log for a read error too.
- A test helper that reads through Dabs' `File.ReadAllText` adds a line end to every line, so compare a read with a read, never with the text that was written.
