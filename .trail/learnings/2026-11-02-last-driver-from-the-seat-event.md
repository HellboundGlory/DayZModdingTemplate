---
title: "A vehicle's last driver comes from the driver-seat event"
author: DayZModdingTemplate
date: 2026-11-02
id: l-33657f55
scope: repo
status: active
src: agent
---

Game build: DayZ 1.30 (1.30.164200).

## Decided
- Record the last driver as whoever last sat in the driver's seat, not whoever started the engine. `OnDriverEnter` is the hook.
- Cars, boats and motorbikes all have it, and CF gives the three script classes the same storage hooks, so patch all three rather than only cars.
- Do not loosen vanilla's attachment or cargo rules for a spawn helper. A newly spawned item is empty, so vanilla's filled-cargo refusal never reaches it.

## Watch out
- No vanilla or CF script calls or overrides `OnDriverEnter`; the engine does. A later probe showed it fires on a retail server for car, boat and bike. Whether a seat switch fires it was not measured.
- On boats and bikes CF returns early from load when the entity is not tracked, so a world saved before CF was installed gives no stored data.

In the code: vanilla `scripts/3_Game/Vehicles/Transport.c:221`; CF `4_World/CommunityFramework/Entities/BoatScript.c:21` and `MotorbikeScript.c:23`.
