# AGENTS.md: {{FAMILY}}

Guide for every coding agent and person working in this repository. The DayZ knowledge is in the `dayz-modder` skill
(`.agents/skills/dayz-modder/`) and the `dayz-*` specialist agents (`.agents/agents/`). Decisions:
`docs/design/decisions.md`, which wins over every plan. The plan: `docs/plan/PLAN.md`. Where code and a recorded
decision disagree, the decision wins; if it looks wrong, ask.

## Facts

- Mod family: {{FAMILY}}. Core mod: `{{CORE}}`. Author: {{AUTHOR}}.
- Target game build: DayZ {{DAYZ_BUILD}}, client and server. Check it again after every game update
  (`docs/modfoundry.md` §When DayZ updates).
- Unpacked vanilla game, read only: `{{DAYZ_REFERENCE}}` (scripts in `scripts/1_Core` to `5_Mission`).
- Workshop folder, read only: `{{WORKSHOP}}`.
- Frameworks the family depends on: {{FRAMEWORKS}}. Read them in their Workshop folders, through ModFoundry where it
  is installed. Never copy them into this repo.
- Reference mods, read for behaviour only (rule 1): {{REFERENCE_MODS}}.
- Layout: `mods/<Id>/` per mod, `tests/<Id>Tests/` per test mod, Rust checks in `tools/`, docs in `docs/`. Shared
  contracts live inside the core mod, because a PBO packs only its mod folder.
- One test server for the family: `server/` at the repo root, ignored by git.
- Versions: every mod is 0.1.0 until its first release, 1.0.0. Change them only with `checks sync-version` (`USAGE.md`).
- ModFoundry (`mf`) is optional: https://github.com/HellboundGlory/ModFoundry-DayZ. Without it the edit hook and
  `check-pbo` skip with a note (`docs/modfoundry.md`).

## DayZ hard rules

1. **A reference is never a source.** Behaviour of another mod can be matched; its code, layouts, JSON files and
   assets are not copied, adapted or paraphrased line by line, none of its files enter this repo, and no class of ours
   `modded class`es one of its classes. Reports describe its behaviour in our own words. Check each reference mod's
   licence before reading it closely.
2. **Not a clone.** Own names, own style, own way of doing things.
3. **Modular.** A mod requires only the family's frameworks and `{{CORE}}`. Links between mods are optional,
   resolved at runtime and null-checked. Every mod installs and works without its siblings.
4. **Everything switchable.** Every player-visible feature has a server off switch, and every value that makes sense
   is a setting.
5. **One name.** A mod id is one token with no underscore: it is the folder, PBO, `CfgPatches` class, class prefix and
   profile folder. The test mod is `<Id>Tests`. Item class and style names may use an underscore after the id.
6. **Frameworks are depended on, never copied or forked.** Read each framework's licence; where it is unclear or
   forbids derivatives, treat it as no-derivatives (for example CC BY-NC-ND 4.0).
7. **Tests and docs never ship.** One test mod per mod. A release check fails if either ends up in a PBO.
8. **RPC only where strictly necessary.** Prefer net-synced variables for entity state, send settings once on
   connect, send on change and never on a timer, batch what can be batched. Every RPC has a row in `docs/design/rpc-list.md`
   with why nothing lighter works.
9. **No real Steam ID in code.** Generated config files use an obvious placeholder and test fixtures use made-up IDs.
   A real one goes only in git-ignored local files.
10. **Player keys are rebindable.** A player-facing key is an input action from an inputs file, set in the game's
    controls menu; no raw `KeyCode` check for one.
11. **Facts come from the exact build.** Every API, signature and number is confirmed in this build's scripts or the
    framework's source before it is used. Wikis and other mods are orientation only.
12. **Reference folders are read only.** Nothing is written into the unpacked game, the Workshop folder or a
    reference mod.
13. **Fast self test.** The in-game self test stays quick: time its checks and make the slow ones cheap or optional.

## Engineering rules

- **Tests first.** Write a failing test, watch it fail, then write the code. Behaviour tests over mocks, no
  duplicates. The default test command finishes in 30 seconds or less; slow checks sit behind an env var.
- **Minimal comments:** a short line only where the why is not obvious. Published mod scripts carry little to no
  comments, a short why at most; tests may have more. Usage notes for mod APIs go in `docs/`.
- **Commits** are small and focused, with a plain imperative subject of about 70 characters, no trailing period, and
  a body only when the why is not obvious. No planning vocabulary (phase, task, ticket ids) in commits or comments.
- **Review before merge.** Every change gets one review before it lands on `main`: a person, or an agent briefed with
  the diff and a reviewer persona from `.agents/personas/`. Log the reviewed range in `docs/review-log.md`.
- **No secrets** in the repo: keys, tokens and real Steam IDs stay in git-ignored local files. Paths are written from
  `~`, never with a home folder's name.
- **Simpler each round.** Before a batch of changes is committed, simplify its range; at each milestone, one
  refactoring pass over the milestone's range (persona `codebase-refactoring-specialist`), each refactor proved by
  identical output before and after.

## Records

- `docs/design/decisions.md`: every decision, with an id, the date and who made it. Answers go there before
  anything acts on them.
- `docs/design/architecture.md`: the module map.
- `docs/plan/PLAN.md` and `docs/plan/specs/`: what is planned, one spec per job (`docs/plan/specs/README.md`).
- `docs/plan/open-items.md`: only what is out of scope now or waits on something else, with what it waits on.
- `docs/review-log.md`: one line per reviewed range.
- `HANDOFF.md`: where things stand, rewritten at the end of each session, under 5,000 characters.
- `CHANGELOG.md`: per release.
- `.trail/learnings/`: lessons the code can't show (optional, with trail).

## Conventions

- Enforce Script traps: `docs/enforce-script.md`. Read it before writing script.
- Specialists: `.agents/agents/dayz-*.md`, read only, rules in `.agents/skills/dayz-modder/specialist-rules.md`.
  `tools/edit-check.sh` runs `mf validate`, `mf lint` or `mf layout check` after each edit of a `config.cpp`, `.c` or
  `.layout` file and warns, never blocks.
- A new class name is searched across `mods/` and `tests/` first.

## Commands

```
cargo test -q --manifest-path tools/Cargo.toml   # the checks' own tests, under 30 s
./setup.sh                                       # once, after "Use this template"
```
