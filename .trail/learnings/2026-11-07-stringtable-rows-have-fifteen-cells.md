---
title: "A stringtable.csv row has fifteen quoted cells and a trailing comma"
author: DayZModdingTemplate
date: 2026-11-07
id: l-0ede0413
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30.

## Decided
Copy the shape of the existing rows exactly: the key plus fourteen language cells, all quoted, then a trailing comma.

## Watch out
Do not count columns with a CSV reader. The header's trailing comma reads as one more column, so rows built from that count are a cell too wide. Count the quoted cells on the row above (`grep -o '"[^"]*"' | wc -l`).
