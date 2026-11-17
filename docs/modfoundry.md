# Using ModFoundry (`mf`) with this template

ModFoundry is a DayZ modding toolchain for agents: one command-line program, `mf`, and an MCP server,
`modfoundry-mcp-server`, that run the same commands. This template works without it, but its checks and its agent
workflow are built around it.

- **Optional.** Nothing in the template needs ModFoundry to read or edit.
- **Linux x86_64 only (tested).** On Windows, every `mf` step in the template skips with a note.
- **Upstream:** <https://github.com/HellboundGlory/ModFoundry-DayZ>. Install and usage live there, so this page links
  to them instead of copying them:
  - [Releases page](https://github.com/HellboundGlory/ModFoundry-DayZ/releases)
  - [INSTALL.md](https://github.com/HellboundGlory/ModFoundry-DayZ/blob/main/INSTALL.md)
  - [USAGE.md](https://github.com/HellboundGlory/ModFoundry-DayZ/blob/main/USAGE.md)
  - [CHANGELOG.md](https://github.com/HellboundGlory/ModFoundry-DayZ/blob/main/CHANGELOG.md) (`mf changelog` prints it)

Written against ModFoundry 2.2.0. `mf <command> --help` is the exact list of flags for the build you have.

## Install and connect an agent

1. Download the release package from the Releases page and follow INSTALL.md. Its `install.sh` puts `mf` and
   `modfoundry-mcp-server` in `~/.local/bin`.
2. `install.sh` registers the MCP server with the agents you pick. In a terminal it asks. Its `--target` values are
   `claude-user`, `claude-project=<dir>`, `codex`, `agents`, `print` (the command line for any other MCP client) and
   one for another coding agent (INSTALL.md lists it). `--yes` takes every agent found, and `--allow-mutations` lets agents change files and launch the game.
   INSTALL.md §Connect an agent has the details.
3. Without `--allow-mutations` the server is read-only: an agent can look at everything and preview changes, not apply
   them. Reads, previews and the checks below work either way.
4. Restart any MCP client that was open during the install.
5. The release also installs the `modfoundry` agent skill (with the `agents` target it goes to
   `~/.agents/skills/modfoundry/SKILL.md`). It tells an agent how to use the tools. Have your agent load it before it
   touches a mod.
6. Run `mf doctor`. It lists the DayZ client, DayZ Server, DayZ Tools and Workshop folders it found, and says when a
   newer release is out. Most commands use what it finds when a path flag is left out.

Updates: `mf update --check` asks, `mf update` installs over a release install. Restart your MCP clients afterwards.
`MF_NO_UPDATE_CHECK=1` turns off the daily check and `mf update`.

## Do not bundle the game or the frameworks

Do not copy vanilla scripts, Community Framework, Dabs or any other mod into your repo. Read them where they already
are. A new mod's dependencies come from the game install and the Steam Workshop folder that `mf doctor` found.

### Save dependencies once

`mf lint`, `mf layout check` and the game check need the mod's dependencies to resolve types.

```sh
mf deps add <mod folder> 1559212036      # Workshop id (all digits), or a mod folder
mf deps list <mod folder>
mf deps remove <mod folder> workshop:1559212036   # exactly as `list` shows it; a folder dependency by its absolute path
```

The saved list lives in ModFoundry's own database on this machine, not in the repo, and a mod folder (yours or a
dependency) is saved by its absolute path. Each clone, worktree or moved folder saves its dependencies again; until
then the edit hook prints "game check skipped" on every edit.

For one run, `--dependency <path>` (repeatable) or `--no-dependencies` overrides the saved list. Over MCP,
`project.dependencies` reads the list and `project.edit_dependencies` changes it. An unknown-type warning (`MF-W3002`)
usually means a dependency is missing.

### Read and search the game and dependency mods

| Task | CLI | MCP tool |
|---|---|---|
| Unpack a PBO, or every PBO in a mod folder, one folder per PBO with `$PBOPREFIX$` | `mf pbo-unpack <input> --out <dir>` (refuses to overwrite) | `pbo.unpack` |
| Search the game's scripts, configs, layouts and your docs in plain words | `mf search "<question>"`; build the index first with `mf search index` | `search.index`, `search.query` |
| Outline a `.c` file or folder | `mf script outline <path>` | `script.outline` |
| Find a class, its definition, its source, hover, completion | | `script.index`, `script.index_status`, `script.find_class`, `script.definition`, `script.source`, `script.hover`, `script.complete` |
| Resolve a config class through its parents and patches | | `config.resolve`, `config.source`, `config.get_tree`, `config.patches` |
| List and read files inside the game or a mod's PBOs | | `vfs.list`, `vfs.read` |
| Turn `config.bin` into text, or the reverse | `mf derapify <config.bin>`, `mf config-bin <config.cpp>` | `config.derapify`, `config.binarize` |

The first `mf search index` downloads a small search model (see INSTALL.md §Where ModFoundry keeps things). Confirm
any API, signature or number in the exact build's scripts before you use it. The wiki and other mods are for
orientation only.

### Unpacking the game

Agents cite the game's own scripts and configs, so keep one unpacked copy per game build, outside the repo, named
after the build, and put its path in `AGENTS.md` §Facts (`setup.sh` asks for it). The game keeps its scripts in
`dta/` and everything else in `Addons/`; unpack both into the same folder:

```sh
mf pbo-unpack ~/.local/share/Steam/steamapps/common/DayZ/dta    --out ~/DayZ-reference/dayz-<build>
mf pbo-unpack ~/.local/share/Steam/steamapps/common/DayZ/Addons --out ~/DayZ-reference/dayz-<build>
```

`dta/` gives `scripts/` (with `1_Core` to `5_Mission`) and `bin/`; `Addons/` gives one folder per PBO, each with
its `$PBOPREFIX$` and `config.cpp` beside `config.bin`. `mf doctor` prints where your game is if it is not the Steam
default.

## Checks on each edit

The template's edit hook runs one check after each edit. It warns and never blocks.

| File edited | Check | What it finds |
|---|---|---|
| `config.cpp` | `mf validate <mod folder>` | config problems, `MF-C*` codes |
| `.c` script | `mf lint <mod folder>` | Enforce Script lint, then the compile errors the game itself would print for the retail server, Diag server and Diag client (about 4 s) |
| `.layout` | `mf layout check <path>` | widget classes, properties and files that do not exist |

Two things to know about `mf lint`:

- Without dependencies (saved or passed) it prints `note: game check skipped` and still says ok.
- A `[diag-client]` finding is a client-only break: retail and the servers never compile `#ifndef NO_GUI` code.

What the game check catches and what it misses: [enforce-script.md](enforce-script.md). Run `mf lint` on the mod and on
its test mod. `mf layout check` also takes `--root` and `--game`.

Use `--preview` on any command that changes files to see the change-set first, and `mf undo` to roll back the newest
one.

## When DayZ updates

Compare builds with `mf game-diff` before you read anything by hand.

```sh
mf game-snapshot                                  # save the installed build for later
mf game-diff <old> <new> --against <mod folder>   # repeat --against for each mod and test mod
```

`<old>` and `<new>` can each be a game install, an unpacked game folder, a `.pbo`, a snapshot `.json` or a saved build
number. The report lists what changed in the game's scripts, configs and layouts, and where your mods use it. One run on
two unpacked builds took 17 s and matched a hand-made list of 29 script changes. Snapshot a build before you update, so
the next compare has an old side. Then recheck the items in [enforce-script.md](enforce-script.md) against the new build.

`mf conflict-check <mod> <mod> ...` takes mod folders in load order and lists what they declare or modify in common:
scripts, configs and packed files (7 s for a handful of mods). It sees overrides, not runtime clashes such as two
handlers set on one chat box.

## Build, run and read logs

These start a real server or client. **The user runs them. An agent only writes the commands, and asks first.**

```sh
mf build <mod folder>                                 # pack into builds/default/@Mod
mf build <mod folder> --authority <key>               # also sign (make a key once with `mf keygen`)
mf run --set <set> --timeout 600                      # DayZServer with the set's mods
mf run-client --set <set> --connect 127.0.0.1:2302 --window 1920x1080 --timeout 300
mf status <run id>                                    # is a recorded run still going, and how did it end
mf errors <profiles folder> --source-root <mod folder>
```

A launch set is a named list of mods in `server/launch-sets.json`. `--set` builds or loads them in order. `--test-mod`
adds a mod for one run only. `mf run` and `mf run-client` also take `--expect-line`, `--fail-line` and `--allow-error`
(each repeatable; `re:` makes a regular expression). `mf run-diag` starts the Diag server and by default the Diag client.

### Boots and runs

- A test mod can run from its last build, not a fresh one. After editing a test file, `mf build` the test mod first, or the game runs the old checks (2026-11-09: a client run showed a fix as
  missing).
- `mf run-client` defaults to a 1280x720 window. Pass `--window` for your screen.
- A lone client sitting at the main menu compiles the client code but runs no checks that start at mission start. For
  client checks, start a server first, then `mf run-client --connect`.
- If the game window is not focused, the game sits on its pause menu: in one run loading took 4 minutes and checks
  started without keyboard focus after a 60 s wait, so a 300 s window closed before the checks finished (2026-11-09).
  Focus the window, or read the result line from the script log after mf gives up.
- `mf run-client` can report `verdict: passed` with failed checks, and an expect-line on a `DONE ... pass= fail=` line
  also matches a `fail=2` one (2026-11-06). Read the summary line in the newest `script_*.log` under the Proton prefix.
  Client-side checks never compile until a client run, so run one before you merge client check code.
- `--expect-line` did not exit after the line was written in mf 1.8.0: the client sat for 10 minutes until stopped by PID
  (2026-11-06). 2.0.0 and later judge a client run by its last mod only and print the errors. Recheck both quirks at
  your next client run.
- mf can refuse a run right after another with `a DayZ client is already running`, with no client open. The Diag
  server runs the client's executable, so wait for the last run's process to go (2026-11-02; inferred). A timed-out
  `mf run-client` leaves the client open, and the next run is refused until it is stopped by PID. `mf` also refuses to
  start a server while a client runs, and a server may die early the same way.
- Closing the Diag client ends `mf run-diag` and stops its server and Proton wrappers (since 2.1.0).
- A test pass of about 2,700 checks needed about 37 seconds to start and about 10 for the checks (2026-11-03). `mf` ends
  the run at the final result line, so an error or crash file written after it, at shutdown, is no longer seen. Past
  the test's own timeout the log just stops with no result line, so read how far it got before suspecting the code.
- A new `crash_*.log` fails mf's verdict unless every block is an allowed script VM exception. Make a test script fail
  on any new crash file too.
- A boot that exited 3 seconds in with code 5 and no script error happened once after a trap was fixed, but a client may
  have been running, so the cause is not proved (2026-11-02).

### Reading errors

`mf errors <profiles folder>` parses RPT and `script.log` files and maps errors to source files. Run it from, or point
`--source-root` at, the folder holding the mod's `config.cpp`.

| Flag | Use |
|---|---|
| `--baseline <folder>` | hide errors a boot without your mods also logs |
| `--learn <id>`, `--meaning`, `--hint` | add a known error with its meaning and fix, printed once per run |
| `--known-errors <file>` | the known-error file (default `server/known-errors.json` under `--source-root`) |
| `--all` | print every error in full, baseline and known ones included |
| `--json` | machine-readable output |

The run summaries of `mf run`, `mf run-client`, `mf run-diag` and `mf status` print errors the same way.

### Server logs

A test server writes logs on every run. Keep only the last two runs' logs, RPTs, ADMs and dumps in its `profiles/`
folder, and remove check folders that boots leave behind. Prune before each test run, and never delete a file written in
the last 15 minutes: a server may still be running. Keep a separate profiles folder for automated test runs, so each one
starts with empty mod logs and you read a test's logs in one known place.

## Reporting gaps

If `mf` lacks a command or a check that would help development, or a check gives a wrong answer, open an issue on
<https://github.com/HellboundGlory/ModFoundry-DayZ>. Include the `mf` version (`mf --version`), the command, and the
smallest case that shows it. An agent that finds such a gap reports it to you instead of working around it silently.

Never publish a mod to the Workshop without the project owner's word. `mf publish` and the other Workshop commands need
a saved rule (`mf agent grant`) for exactly this reason.
