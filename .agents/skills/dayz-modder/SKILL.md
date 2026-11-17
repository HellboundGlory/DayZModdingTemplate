---
name: dayz-modder
description: DayZ modding in this repo. Use before writing or reviewing Enforce Script, config.cpp, layouts, inputs, stringtables, types.xml or anything a mod saves; before adding a mod; and when a boot, self test or client run fails.
---

# DayZ modder

This repo is a DayZ mod family. Its facts (game build, unpacked game, Workshop folder, frameworks, core mod) are in
`AGENTS.md` §Facts, and its rules in `AGENTS.md` §DayZ hard rules. Read both first.

## Before writing script

1. Read `docs/enforce-script.md`: what `mf lint` catches, and the traps it misses. The second list is the one that
   costs boots.
2. Confirm every API in the target build's own scripts (the unpacked game in §Facts) or in the framework's source,
   never from memory, a wiki or another mod. With ModFoundry: `mf search`, the `script_*` and `config_*` MCP tools.
   Without it: grep the unpacked game.
3. Search `mods/` and `tests/` for a new class name before adding it, and check it against the frameworks' class
   names too: a method named like a framework class is read as that class's constructor.
4. Put code in the right script layer (`3_Game`, `4_World`, `5_Mission`). Client-only code sits inside
   `#ifndef NO_GUI`; servers never compile it, so only `mf lint`'s `[diag-client]` check or a client run sees it.

## Which specialist to ask

The `dayz-*` agents are read only and answer from the exact build. Brief one with the question and the files.

| Question | Agent |
|---|---|
| How a script API, modded class, layer or call order works | `dayz-enforce-script` |
| config.cpp, CfgPatches, requiredAddons, load order, stringtables, inputs | `dayz-config` |
| Layouts, widgets, imagesets, styles, fonts | `dayz-ui-layout` |
| OnStoreSave and OnStoreLoad, JSON settings, profile files | `dayz-persistence` |
| types.xml, events.xml, cfggameplay.json, serverDZ.cfg | `dayz-server-economy` |
| A failed boot, self test or client run | `dayz-log-debugger` |
| A diff before it merges (the DayZ lens of the review) | `dayz-mod-reviewer` |

Shared rules for all of them: `specialist-rules.md` beside this file.

## Adding a mod

`USAGE.md` §Adding a mod has the steps: the folder under `mods/`, its test mod under `tests/`, `mf deps add` for its
dependencies, and the server link.

## Checks

- The edit hook (`tools/edit-check.sh`) runs `mf validate`, `mf lint` or `mf layout check` on each edited
  `config.cpp`, `.c` or `.layout` file. It warns and never blocks. "note: game check skipped" means the mod's
  dependencies are not saved: `mf deps add`.
- The Rust checks in `tools/` run with `cargo run --manifest-path tools/Cargo.toml -- <check>`; `USAGE.md`
  lists them. Only `check-pbo` needs ModFoundry, and it skips with a note without it.
- ModFoundry usage, test runs and game updates: `docs/modfoundry.md`.

## When something fails in game

Read the newest script log and RPT first (`dayz-log-debugger` knows where they are). Find the cause before changing
code: state the mechanism in one sentence and check the change removes it. A trap that cost a boot gets a line in
`docs/enforce-script.md`, with its evidence and date.
