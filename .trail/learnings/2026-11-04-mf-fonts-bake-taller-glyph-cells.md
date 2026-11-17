---
title: "Fonts baked by mf draw small because their glyph cells are taller"
author: DayZModdingTemplate
date: 2026-11-04
id: l-936e5e53
scope: repo
status: active
src: agent
---

Measured with mf 1.5.0 on 2026-11-04; recheck on newer mf. Reported upstream; not known to be fixed.

## Watch out
A baked font's `HEAD` size and its glyph cell height differ. A 12 px bake had cells 19 px tall (`TCRD` h = 0x13), while vanilla `Metron12.fnt` has 12. The engine seems to fit the cell into the exact size (inferred from measurements, not engine source), so letters shrink to about 12/19 and thin strokes (l, i) vanish. A 13 px body text drew about 7 px from top to descender in a 1920x1080 screenshot, while vanilla `robotomono_msdf_` text at the same exact size drew right.

## Tried and ruled out
Blaming the scale code. It asked for 13 as intended.

In the code: `.fnt` files are FNT3 chunks HEAD, GLPS, TCRD, KERN; TCRD records are 12 bytes (x y w h ...). `SetTextExactSize` sets the size.
