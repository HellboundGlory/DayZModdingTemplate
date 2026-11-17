# Architecture

The module map: one row per mod, what it owns, what it exposes, and what it needs. Mods reach each other only through
the core mod's contracts, resolved at runtime and null-checked (`AGENTS.md` rule 3).

| Mod | Owns | Exposes | Needs |
|---|---|---|---|

Inside a mod, game and engine calls stay at the edges: the rules and data live in plain classes that tests can drive
without the engine, and the `modded class` hooks only translate between the game and those classes.
