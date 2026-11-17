---
title: "Imageset entry names use underscores, not hyphens"
author: DayZModdingTemplate
date: 2026-11-04
id: l-294b10b4
scope: repo
status: active
src: agent
---

Measured with mf 1.5.0 on 2026-11-04; recheck on newer mf.

## Decided
Name imageset entries with letters, digits and `_` only. `mf imageset` refuses anything else, so an icon file `chevron-down.png` becomes the entry `chevron_down`, and layouts must say `image:chevron_down`.

## Watch out
- mf writes `mpix 1` for the base texture and `mpix 2` for `@2x`. Vanilla's dayz_gui uses 0 and 1; playstation_buttons and Dabs use 1 and 2.
- Its edds output is uncompressed 32-bit, so a gradient mask does not band.
- `mf image --out <existing folder>` fails with "virtual path invalid"; pass a file path.
- A byte-exact bake check holds only with the same Inkscape, mf and Pillow versions.
