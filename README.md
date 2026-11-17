# DayZModdingTemplate

A starting point for a DayZ mod family that coding agents work on.

The template gives a coding agent (and you) what it needs to write DayZ mods well from the first day:

- `AGENTS.md`: the DayZ hard rules and the engineering and record-keeping rules. Claude Code, Codex and most other
  agents read it on their own.
- The `dayz-modder` skill and seven read-only `dayz-*` specialists (script, config, UI layouts, saved data, server
  economy, log reading, review), all told to confirm every API in your exact game build.
- `docs/enforce-script.md`: Enforce Script traps found by boots and probes on DayZ 1.30, split into what ModFoundry's
  `mf lint` catches and what it misses.
- An edit hook that runs ModFoundry's checks on every `config.cpp`, `.c` and `.layout` an agent edits, and Rust
  checks for the whole family in `tools/`.
- Empty `mods/` and `tests/` folders, a decisions log, a plan, a review log and a handoff.

It holds no mods and no code examples. Framework and reference mods are read where Steam installed them, through
[ModFoundry](https://github.com/HellboundGlory/ModFoundry-DayZ), not copied in.

## Start

1. On GitHub, press **Use this template**, create your repo, and clone it.
2. Install what `PREREQUISITES.md` lists.
3. Run `./setup.sh` in the clone (Git Bash on Windows). It asks for your mod family's name, prefix, core mod, author,
   git identity, target DayZ build, frameworks, reference mods and game paths, fills in the placeholders, and writes `tools/family.conf` for the checks.
4. Add your first mod: `USAGE.md` §Adding a mod.

Tested on Linux only. The setup script is plain bash and should run in Git Bash on Windows, but that is untested,
and ModFoundry itself is Linux only, so on Windows every `mf` step skips with a note.

## Licence

MIT, in `LICENSE`. The two personas in `.agents/personas/` keep their own MIT licence and notice.
