---
title: "Community DayZ agent kits can carry wrong rules for 1.30"
author: DayZModdingTemplate
date: 2026-11-10
id: l-711d5f36
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30. Two public agent kits were read on 2026-11-07 and 2026-11-10.

## Watch out
- One kit was written for DayZ 1.28 and says "never use delete" (vanilla 1.30 has 136 uses) and "Enforce is garbage collected". Both are wrong as stated. Its licence file (GPL v3) also disagreed with its readme (MIT).
- The other assumes a Windows `P:\` work drive and DayZ Tools, embeds queries through a paid cloud API, and its shared rules mark 23 claims unverified. It cites vanilla line numbers with no game build.
- Use such kits for ideas only. Check every API, rule and line number in the source of your pinned build.
