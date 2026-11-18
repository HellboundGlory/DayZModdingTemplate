# Using the template

## Adding a mod

Mod ids are one token with no underscore (`AGENTS.md` rule 5). For a mod `MyId`:

1. Scaffold it (ModFoundry 2.3.0 or later; with an older one, or without ModFoundry, write `config.cpp`, `mod.cpp` and
   the script folders by hand and make the edits in step 5 yourself):
   `mf new MyId --dir mods/MyId --author "<you>" --patch-class MyId --required-addon <patch>`, one
   `--required-addon` for each CfgPatches name `tools/family.conf` lists for a framework (the core mod), or for the
   core's own name (any other mod). It writes the `CfgPatches` class as `MyId` (one name, rule 5), the
   `requiredAddons`, a `version` line in `CfgMods` and a starter `Scripts/5_Mission/MissionServer.c`; add
   `--no-starter` when the mod patches no mission class.
2. Add its test mod the same way, listing its own mod:
   `mf new MyIdTests --dir tests/MyIdTests --patch-class MyIdTests --required-addon MyId --no-starter`. The test mod
   never ships.
3. Save the dependencies `mf lint` and the game check use, for both:
   `mf deps add mods/MyId <framework Workshop id or folder>` for each framework, plus `mods/<core mod>` for any mod
   that is not the core; the test mod also gets `mods/MyId`. The list goes into `.modfoundry/dependencies.json` in each
   mod (Workshop ids, and folders relative to the mod), so commit it: a new clone or worktree has the dependencies at
   once. `mf deps list mods/MyId` shows them (`docs/modfoundry.md`).
4. The shared test server: once, create `server/` at the repo root with your `serverDZ.cfg` (git ignores it). In each
   of the two mod folders, link it with `ln -s ../../server server` and keep it out of the PBO with a `.pboignore`
   file holding the line `/server/` (`mf new` does not make one). `mf run --help` explains launch sets.
5. Finish what the family checks want:
   - A file holding `modded class X` is named `X.c` (the scaffold's starter is already `MissionServer.c`) and opens
     with four `//` lines: `// Why:` the reason, then what breaks without it, then `// Layer:` and why the class sits
     in that script layer, then which other mods patch the class. Give each patched class a row in
     `tools/modded-allowlist.txt`. The starter has neither, so `check-modded` fails until you add them or delete it.
   - `sync-version` keeps the `version` line in `CfgMods` in step with `VERSION`.
   - Made by hand, or by an `mf new` older than 2.3.0: the `CfgPatches` class named as the mod (the test mod's
     `MyIdTests`), the `requiredAddons` of step 1, that `version = "0.1.0";` line inside the `CfgMods` class, beside
     `type`, and a starter named `MyIdMission.c` renamed to `MissionServer.c` (or deleted).
   Then run the checks (§Running the checks); each should print `OK`.
6. Add a `## <ModId>` heading for its decisions in `docs/design/decisions.md` and a row in
   `docs/design/architecture.md`.

## Running the checks

- On every edit: the hook in `.claude/settings.json` runs `tools/edit-check.sh` (warns, never blocks).
- By hand: `mf lint mods/MyId`, `mf validate mods/MyId`, `mf layout check mods/MyId`.
- The family checks: `cargo run -q --manifest-path tools/Cargo.toml -- <check> [args]`; `-- --help` lists them.
- Their own tests: `cargo test -q --manifest-path tools/Cargo.toml`. `CHECKS_REAL_REFERENCE=1` adds a slow test
  that runs check-reference on the folders in your `tools/family.conf`.

| Check | What it checks | Arguments |
|---|---|---|
| `check-boundaries` | each mod's code names only its own prefix, the core and its contracts | `[--root R]` |
| `check-dependencies` | each mod's `requiredAddons` lists only what it may, and its patch classes are its own | `[--root R]` |
| `check-inputs` | inputs files: actions, sorting, presets, loc keys and the keys scripts look up | `[root]` |
| `check-modded` | every `modded class` has a row in `tools/modded-allowlist.txt` (tab separated: ModId, class, owner, reason), and each patch file is named and headed | `[--root R]` |
| `check-pbo` | a mod would not pack its docs, tools or tests; skips without ModFoundry | `<mod root>` or `--list-file F` |
| `check-perf-static` | frame callbacks and RPC handlers do no file work, hold no unbounded loop and cap what they read | `[--root R]` |
| `check-reference` | no mod uses a name the reference mod declares (`AGENTS.md` rule 1); skips with no `reference` line | `[--root R] [--reference D] [--vanilla D]` |
| `family` | shows the values the checks read from `tools/family.conf` | `[--root R]` |
| `prune-server-logs` | keeps the last test-server runs' logs and removes check folders (files under 15 minutes old stay) | `[--keep N] [--root R] <profiles>` |
| `sync-version` | every mod's version is in step with `VERSION`; also sets or bumps them | `[--root R] [--check \| --set X.Y.Z \| --bump MOD PART]` |

Checks that read `mods/` skip with a note while it holds no mod. A folder that `tools/family.conf` or a flag names but
that does not exist is an error (exit 2).

The family checks read `tools/family.conf`, which `setup.sh` writes; `-- family` prints what they read. One
`key = value` per line, `#` comments on their own line, a relative path under the repo root and `~` for your home:

```
prefix = Acme
core = AcmeCore
framework = CF JM_CF_Scripts
framework = Dabs DF_GUI DF_Scripts
reference = ~/reference/SomeMod
vanilla = ~/DayZ-reference/dayz-1.30.164289
```

A `framework` line is the framework's name, then the CfgPatches names a mod may require from it. `reference`, the
unpacked mod that check-reference compares against, is optional; without it that check skips.

Test runs (server, client, self test) and game updates: `docs/modfoundry.md`.

## Without ModFoundry

| Step | Without mf |
|---|---|
| Edit hook | says the file was not checked |
| `check-pbo` | skips with a note, exit 0; the other checks do not use mf |
| Specialists | read the unpacked game with Read and Grep, and say the mf checks did not run |
| Building and test runs | use DayZ Tools or your own scripts |

## Agents

- `AGENTS.md` is the rule file every agent reads.
- `.agents/skills/` and `.agents/agents/` are the sources. Claude Code reads only `.claude/skills/` and
  `.claude/agents/`, so `setup.sh` copies them there; run `./setup.sh --sync-agents` after changing a source, and
  `./setup.sh --check-agents` to see whether a copy differs from its source (exit 1).
- `.agents/personas/` holds the reviewer and refactoring personas the engineering rules name.
