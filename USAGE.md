# Using the template

## Adding a mod

Mod ids are one token with no underscore (`AGENTS.md` rule 5). For a mod `MyId`:

1. Scaffold it: `mf new MyId --dir mods/MyId --author "<you>"` (or write `config.cpp`, `mod.cpp` and the script
   folders by hand without ModFoundry).
2. Add its test mod the same way: `mf new MyIdTests --dir tests/MyIdTests`, then add `"MyId"` to the
   `requiredAddons` of its `config.cpp` by hand (`mf new` lists only `DZ_Data`). The test mod never ships.
3. Save the dependencies `mf lint` and the game check use, for both:
   `mf deps add mods/MyId <framework Workshop id or folder>` for each framework, plus `mods/<core mod>` for any mod
   that is not the core; the test mod also gets `mods/MyId`. `mf deps list mods/MyId` shows them. They are saved on
   this machine by absolute path, so a new clone or worktree saves them again (`docs/modfoundry.md`).
4. The shared test server: once, create `server/` at the repo root with your `serverDZ.cfg` (git ignores it). In each
   of the two mod folders, link it with `ln -s ../../server server` and keep it out of the PBO with a `.pboignore`
   file holding the line `/server/` (`mf new` does not make one). `mf run --help` explains launch sets.
5. Add a `## <ModId>` heading for its decisions in `docs/design/decisions.md` and a row in
   `docs/design/architecture.md`.

## Running the checks

- On every edit: the hook in `.claude/settings.json` runs `tools/edit-check.sh` (warns, never blocks).
- By hand: `mf lint mods/MyId`, `mf validate mods/MyId`, `mf layout check mods/MyId`.
- The family checks: `cargo run -q --manifest-path tools/Cargo.toml -- <check>`; `-- --help` lists them.
- Their own tests: `cargo test -q --manifest-path tools/Cargo.toml`.

Test runs (server, client, self test) and game updates: `docs/modfoundry.md`.

## Without ModFoundry

| Step | Without mf |
|---|---|
| Edit hook | says the file was not checked |
| Checks that read the game or pack a PBO | skip with a note, exit 0 |
| Specialists | read the unpacked game with Read and Grep, and say the mf checks did not run |
| Building and test runs | use DayZ Tools or your own scripts |

## Agents

- `AGENTS.md` is the rule file every agent reads.
- `.agents/skills/` and `.agents/agents/` are the sources. Claude Code reads only `.claude/skills/` and
  `.claude/agents/`, so `setup.sh` copies them there; run `./setup.sh --sync-agents` after changing a source.
- `.agents/personas/` holds the reviewer and refactoring personas the engineering rules name.
