# Rules for the DayZ specialist agents

The agents in `.agents/agents/dayz-*.md` read this first. They answer questions and review work. They are read-only and report findings: they never edit, build, launch or commit anything.

The project's own facts live in `AGENTS.md` §Facts: the target DayZ build, where the unpacked vanilla game is, the Workshop folder, the frameworks the mod family depends on (none are assumed; Community Framework and Dabs are common choices), and the core mod id. Each mod's id is also its class prefix (`AGENTS.md` rule 5). Read them there and never guess a path, a build number or a mod id. If a fact is missing from §Facts, say so and ask.

## Sources, in this order

1. Vanilla DayZ at the target build in `AGENTS.md` §Facts, unpacked at the path it gives (`scripts/1_Core` to `5_Mission`, `gui/layouts/`; each PBO's folder names its game path in `$PBOPREFIX$`).
2. The frameworks `AGENTS.md` §Facts lists, read from their Workshop folders. They are packed, so read them through the mf tools below (`vfs_list`, `vfs_read`, `script_source`).
3. Vanilla mission files (types, events, globals, gameplay JSON): the DayZ Server install's `mpmissions/` folder (`mf doctor` prints where the server is installed).
4. The project: `AGENTS.md`, `docs/working-notes.md`, `docs/enforce-script.md` (what `mf lint` catches and the traps it misses), `docs/design/decisions.md` by id, and the other files in `docs/design/`.
5. The ModFoundry (mf) MCP tools. To find where vanilla, a framework or the project docs do something, start with `search_query` (plain words; `kind` and `source` narrow it) before any Grep sweep, then read the hits it names. For exact lookups build the index once (`script_index` with `root` set to the mod folder), then `script_find_class`, `script_definition`, `script_hover`, `script_source`, `script_outline`; for configs `config_index`, then `config_resolve` and `config_source`; for the economy files `ce_*`. A `.layout` question or change: `layout_check`, with the frameworks as dependencies. What several mods override in common: `conflict_check` in load order (the frameworks, then any reference mods named in `AGENTS.md`, then our mod). What a game update changed: `game_diff` with our mods as `against`. Server and client errors: `diag_errors` with `source_root` set to the mod folder; known errors print once with their meaning.
6. The DayZ Modding Wiki (https://stardz-team.github.io/DayZ-Modding-Wiki/en/) for orientation only. Never cite it as proof.

If the modfoundry tools are missing, fall back to Read, Grep and Glob over the unpacked game and the dependency mods' folders named in `AGENTS.md` §Facts, and say in the answer that the mf checks were not run.

Any reference mod the project names in `AGENTS.md` may be read to learn behaviour. Describe what it does in your own words; never quote it, paraphrase it line by line or suggest copying its code, layouts, JSON or assets.

## Every answer

- Each fact about the game or a framework gives `file:line` in the source above it came from, and says how it was checked: read in source, found by an mf tool, or not verified. A claim you could not check is labelled a guess.
- Line numbers are for the target build in `AGENTS.md` §Facts unless you say otherwise.
- Short: the answer first, then the citations, then what is still unknown. No essays and no code longer than the question needs.
- Where the project's decisions or `AGENTS.md` settle a point, they win; cite the decision id. If one looks wrong, say so rather than working around it.
- Hold to the project's hard rules in `AGENTS.md` in anything you suggest: modular (a mod needs only its declared frameworks and the family's core mod), a server off switch per player-visible feature, one name per mod, RPC only where nothing lighter works, rebindable input actions rather than raw `KeyCode`, little to no comments in published mod scripts (a short why at most).
