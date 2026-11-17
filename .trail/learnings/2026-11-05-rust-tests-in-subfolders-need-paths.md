---
title: "Rust integration test modules in a subfolder need explicit paths"
author: DayZModdingTemplate
date: 2026-11-05
id: l-d3c449f1
scope: repo
status: active
src: agent
---

## Decided
Put the modules of one integration test binary in a subfolder, and declare each with an explicit `#[path = "sub/<name>.rs"] mod <name>;` in the binary's root file.

## Tried and ruled out
- A plain `mod <name>;` in `tests/sub.rs`: the build fails with E0583 "file not found", because the crate root looks for `tests/<name>.rs`.
- Filtering `cargo test` with a `sub::` prefix: it matches nothing and still reports "ok" with 0 tests. Test paths start at the binary's root, so filter with `<module>::<test>`.

## Watch out
Always read the "N passed" count of the binary you meant to run. A filter that matches nothing exits 0.
