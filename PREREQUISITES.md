# Prerequisites

| What | Why | Needed |
|---|---|---|
| git | the repo | yes |
| bash 4 or newer (Git Bash on Windows) | `setup.sh` and the edit hook | yes |
| jq | the edit hook reads the agent's edit event with it (Git Bash has none; install it separately) | for the hook |
| Rust (stable, through [rustup](https://rustup.rs)) | the checks in `tools/` | for the checks |
| DayZ and DayZ Server through Steam | the game build the mods target; the test server | yes |
| [ModFoundry](https://github.com/HellboundGlory/ModFoundry-DayZ) (`mf` and its MCP server) | reading the game and other mods, the edit-time checks, building, test runs | optional, Linux x86_64 only |
| [trail](https://github.com/trailhq/Trail-Code) (`npm install -g @trailhq/trail-code`) | carries the lessons in `.trail/learnings/` into each agent session | optional |

ModFoundry: download the newest package from its
[Releases page](https://github.com/HellboundGlory/ModFoundry-DayZ/releases) and follow its
[INSTALL.md](https://github.com/HellboundGlory/ModFoundry-DayZ/blob/main/INSTALL.md); `install.sh` registers the MCP
server with your agents. Then run `mf doctor` to see the DayZ and Workshop folders it found. Use ModFoundry 2.3.0 or
later: the steps in `USAGE.md` need its `mf new` flags and the dependency file in each mod; `mf update` installs the
newest release. Without ModFoundry the template still works: the edit hook and `check-pbo` say so and skip.

A coding agent: Claude Code reads `AGENTS.md`, `.claude/skills/` and `.claude/agents/` (which `setup.sh` fills from
`.agents/`). It reads `AGENTS.md` only when there is no `CLAUDE.md` in the folder or above it; if you keep one, have
it point at `AGENTS.md`. Other agents read `AGENTS.md` and, where they support it, `.agents/`.
