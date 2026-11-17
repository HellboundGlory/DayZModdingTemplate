---
title: "A UI mask texture's sweep must be in its colour, not its alpha"
author: DayZModdingTemplate
date: 2026-11-05
id: l-893018eb
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30. Seen in a client on 2026-11-05.

## Decided
Bake a mask texture with the sweep ramp as grey shades in RGB and full alpha. A ring mask with the ramp only in alpha and white RGB stayed empty during a hold and appeared all at once at the end, although the code feeding `SetMaskProgress` was right.

## Watch out
Vanilla masks (`cardial_small_mask.edds`, `cardial_big_mask.edds`) keep the sweep in grey RGB with alpha 255. The engine seems to read the colour channels, so with an alpha-only ramp it saw every pixel at the last step. Checked by decoding the uncompressed mips of both files.

In the code: `LoadMaskTexture` and `SetMaskProgress` on `ImageWidget`.
