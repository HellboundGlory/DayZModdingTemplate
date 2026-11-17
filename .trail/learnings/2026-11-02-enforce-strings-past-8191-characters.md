---
title: "Enforce string functions break past 8,191 characters"
author: DayZModdingTemplate
date: 2026-11-02
id: l-4609b7c5
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Decided
Read files that can grow with `FGets` and join the lines in chunks. Never cut a long body with `Replace` or a long `Substring`. To change one row of a growing file, append a new row and let the last one win.

## Tried and ruled out
- `Replace` on a body past 8,191 characters: it cuts the string to what it found in the first part, and empties it if it found nothing.
- `Substring(0, long)`: returns at most 8,191 characters.
- Walking lines with `IndexOfFrom`: 10.9 million ticks for 12,000 rows, against about 88,000 for `FGets` on 14,634 rows.

## Watch out
None of these raise an error and a linter sees nothing. A short `Substring` from far along, `+`, `==`, `!=`, `Length`, `IndexOf`, `LastIndexOf`, `FPrint` and `ReadFile` all worked at up to 600,000 characters. Test any file format at a realistic size in a boot.
