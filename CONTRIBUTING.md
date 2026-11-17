# Contributing

- Work on a branch; `main` only takes reviewed changes.
- Tests first: a failing test, then the code. The default test command stays under 30 seconds.
- Commits: small, one change each, a plain imperative subject of about 70 characters with no trailing period, and a
  body only when the why is not obvious.
- Comments: little to no comments in published mod scripts (a short why at most); elsewhere a short line only where
  the why is not obvious.
- Before a merge, one review of the range: a person, or an agent given the diff and
  `.agents/personas/adversarial-code-reviewer/PERSONA.md`, plus `dayz-mod-reviewer` for script, config or layout
  changes. Fix what it finds, then add one line to `docs/review-log.md`.
- A decision goes in `docs/design/decisions.md` before the code that acts on it.
- Never commit secrets or real Steam IDs; they go in git-ignored local files. Write paths from `~`.
