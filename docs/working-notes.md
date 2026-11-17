# Working notes

Practical notes for anyone working in this repo, person or agent. The rules are in `AGENTS.md`; the Enforce Script
traps in `docs/enforce-script.md`.

## Working on a job

- Read the job's spec, the decision ids it cites and the files it owns. Change only those files; anything else that
  needs a change goes in your report.
- Start from a failing test. Run the default tests before and after, and give the time.
- Check every API in the target build's scripts or the framework's source before using it, and say in the report how
  each claim was checked: by a build or test, by reading the source, or not at all.
- Never launch the game, a server or a client unless the person running the work asks you to; list what needs an
  in-game check instead.
- Never commit unless asked.
- Before reporting, review your own diff with `.agents/personas/adversarial-code-reviewer/PERSONA.md` and, for script,
  config or layout changes, the `dayz-mod-reviewer` agent. Fix what they find.

## Report

What changed (files), the test command and its result and time, how each API claim was checked, what is left for an
in-game check, and anything outside your files that needs a change.

## Tips

One line per trap or slip that cost time, with its evidence and date. A trap in Enforce Script itself goes in
`docs/enforce-script.md` instead.
