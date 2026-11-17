---
title: "A returned array stored in a ref field crashed the server"
author: DayZModdingTemplate
date: 2026-11-06
id: l-3eaa3b9b
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30. Seen in a server boot on 2026-11-06.

## Decided
In test case classes keep array data as text, or take arrays held in a caller's local, and build the arrays inside the check loop.

## Tried and ruled out
Passing a helper's returned `new array<int>` straight into a case constructor and storing it in a `ref` field. The server crashed with an unhandled exception at the store on the first boot that ran it. Lint and the build both passed it. The exact engine mechanism is not proven.

## Watch out
Only a boot shows this. Client-only checks need a client run too, since a server boot does not compile `#ifndef NO_GUI` code.
