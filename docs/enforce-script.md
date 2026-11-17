# Enforce Script: what `mf lint` catches, and what it does not

These notes were found on DayZ 1.30 (builds 1.30.164200 and 1.30.164289) with ModFoundry 2.1.0's `mf lint`.
ModFoundry 2.2.0 added no new catches: its changelog lists `mf deps` (saved dependencies), `mf update` and release
packages. Each item is rechecked when a new game build comes out, so treat a date below as the day it was seen, not
as a promise for later builds.

"Probe" means a small throwaway test mod. "Boot" means a server or client start. "Read in source" means the trap was
read in the game's or a framework's scripts and not seen failing. Say which one you have when you add a line.

## What `mf lint` catches

Since ModFoundry 2.1.0, `mf lint` runs the game's own compiler for the retail server, the Diag server and the Diag
client. Three things decide whether the game check really ran:

- **Dependencies.** The mod's framework mods (Community Framework, Dabs, and any other mod yours needs) must be known to
  `mf lint`. Since 2.2.0 (its changelog), save them once with `mf deps add <mod> <dependency>`, or pass them for one run with
  `--dependency <mod folder>`. Without any, the CLI printed `note: game check skipped` and then ok (2.1.0,
  2026-11-16). Over MCP, `script_lint` with an empty dependency list ran the game check, which failed the module on
  every build while listing no findings (2026-11-17). Read the game-check lines, not the exit code or the findings
  count.
- **`[diag-client]` findings.** A finding with this tag is a client-only break. The retail build and the servers never
  compile code inside `#ifndef NO_GUI`, so only the Diag client sees it.
- **Test mods.** Run it on the mod and on its test mod, each with its dependencies.

Caught, and listed here only so a boot message reads familiar (probed 2026-11-16):

- a variable declared twice in one function
- a non-void function without a final `return`
- shift or mask on a float
- `n + " at"` (`Incompatible parameter`)
- a binary operator starting a continuation line
- a field name its base class already uses
- more than 16 parameters
- `out`, `local`, `owned` or `set` as a name
- `new` on a class with a private constructor
- a field without `ref` given `new`, or a parameter given `new`
- a class declared twice
- a `foreach` straight over a method's return value (`MF-W3005`, a warning; checked 2026-11-17 with 2.2.0). In
  game the first such loop in a run throws "NULL pointer to instance" into `crash_*.log` (probe boots, 2026-11-10), so
  hold the result in a local first
- a server-side call or callback naming a method inside `#ifndef NO_GUI` (both servers)
- `[diag-client]` only: a misspelt type, a static call on the wrong class, `TextWidget.GetText()`, `GetTextSize` on a
  plain `Widget`, and a `ClassName()` method

A bare call to a method named after a class (`Step()`, `Building()`) is `MF-E3004` outside the guard.

Two print only a warning on the retail server and fail both Diag builds: a subclass constructor naming its parameters
unlike the base's, and a method named `Type()` (Dabs calls the built-in from `ScriptedViewBase`).

## Enforce Script rules `mf lint` does not catch

The game check passes these. Only a boot, a client run or a careful read finds them.

### Language and runtime

- A class constant read from a static method is `static const`, not plain `const`.
- A function returning `typename` cannot `return null`. Return a `bool` with an `out typename` instead.
- `Math.Round` rounds a half to the even neighbour. A test boot on 2026-11-07 gave 454 for 454.5, 236 for 235.5 and 134
  for 133.5. An expected size worked out by hand rounds halves to even, not up.
- A templated class never extends a templated class with its own parameter. Extend with a concrete type or pass a plain
  helper object.
- A static method's parameter named like a field of its class reads as that field: `static Make(string title)` with
  `note.title = title` failed "Trying to access non-static member". Name the parameters differently (test boot,
  2026-11-07).
- A NaN passes `!(x > 0)`, because the comparison reads true for NaN here. Test it with `x != x || x <= 0`, which a boot
  proved (2026-11-09).
- `ray * (step * SOME_INT)`, a vector times a bracketed product of ints, is `Incompatible parameter` on the constant,
  and `mf lint` passes it, as `forward * 8` is fine. Read the product into a `float` first (boot, 2026-11-03).
- `array<ref X>.InsertAll` from another `array<ref X>` is refused ("array<@Param> and array<Param> are unrelated", boot,
  2026-10-31), and `mf lint` passes it. Insert in a loop.
- A method named `Queue` is refused when called bare (`Too many parameters for 'Queue' method`, boot, 2026-11-02), and
  `mf lint` passes it. The clash is with Dabs' `Queue<Class T>` class (`DabsFramework/!Core/Queue.c:1`), read as its
  constructor, not an inherited method (final review, 2026-11-04). Check a new method name against Dabs and CF classes
  too.
- A method named `Selection` (vanilla `class Selection`, private constructor) called bare inside `#ifndef NO_GUI` passes
  `mf lint`: `MF-E3004` sees bare constructor calls only outside the guard (probe, 2026-11-16).
- A `ref UAIDWrapper` field fails the client compile (private destructor, `UAInput.c:27`; hold it without `ref`), and the
  game check passes it on every build (first client build, 2026-11-04; probe, 2026-11-16).
- A duplicate class name across a mod and its test mod is caught only when the other mod is passed as a dependency.
  Still grep for `class <Name>` in every mod before naming a new class (2026-11-09).
- `modded class` overrides always call `super` and never change what the vanilla method returns for cases they don't
  handle.
- `EnScript.SetClassVar` returned 0 on success here, against its doc comment's `success = 1` (`EnScript.c`, boot,
  2026-11-02). Don't trust its result. Read the value back to check.

### Objects, temporaries and `foreach`

- Hold a returned object in a local before calling a method on it: `!GetData().Item(7).IsEnabled()` (a getter
  returning a fresh object, then a method on it) crashed the server on 2026-11-07 (unhandled exception at that line),
  since the fresh object can be freed before the call. `mf lint` and the test boot passed it. Again on 2026-11-09:
  `GetHost().SetHidden(id, !IsShown())` stopped with "NULL pointer to instance" in a client run, though
  `GetHost().Has(id)` on the line above worked.
- `bool x = one && list.Get(0).IndexOf(y) == 0` read false with both sides true (boot, 2026-11-02), so read a method call
  into a local before any `&&`.
- Don't put two method calls joined by `&&` straight in a call's argument: `report.Check("x", a.IsRefused() &&
  a.Calls() == 0)` passed false with both sides true (boot, 2026-10-31). Read them into locals first. The exact trigger
  wasn't isolated.
- A function's returned `new array` passed straight into a constructor and stored in a `ref` field crashed the server at
  the store (a test case, boot, 2026-11-06), and `mf lint` passes it. Hold the array in a local first, or keep case data
  as text and build arrays in the loop.
- A plain local, a non-`ref` map value or a non-`ref` array element doesn't own an object. `CF_ModStorageMap` and
  `ScriptView.All` hold weakly, so keep what you put there in a `ref` field or `array<ref X>` (final reviews, 2026-11-01;
  read in source, not seen failing).
- `Object.Delete()` only queues `ObjectDelete` on the call queue (`Object.c:84`), so in the same frame the object still
  reads its type and `IsSetForDeletion()` is still false. A check that a delete happened needs a later frame
  (2026-11-04).

### Which side am I on, and when

- Only `IsDedicatedServer()` is right while `DayZGame` is being built. A client there reads `IsClient()` false,
  `IsServer()` true and `IsMultiplayer()` false; a dedicated server reads `IsMultiplayer()` false. All four are right by
  the mission's constructor (probe boots, 2026-11-10). A helper built on `IsMultiplayer() && !IsServer()` is therefore
  only right from mission start.
- Naming a class from `5_Mission/GUI` (for example `IngameHud`) in a `5_Mission` file crashes the Diag server while the
  Mission module loads, because the server compiles `NO_GUI` and has no such class.
- A Dabs menu's close queues a cursor hide on the GUI queue. Closed at mission end, it lands on the main menu and kills
  it, so remove the call in `OnMissionFinish` (first session, 2026-11-01).

### Files, strings and JSON

- Enforce has no file rename, and vanilla's `FileRename` is a copy and a delete (`PluginFileHandler.c:13`). There is no
  atomic replace (read in source, 2026-11-01). A write that survives a kill: delete any stale `<file>.tmp`; write
  `<file>.tmp`, close it and confirm it is whole (compare its bytes or byte count with what was written); if the live
  file is good, copy it to `<file>.bak`; copy the `.tmp` over the live file, and delete the `.tmp` only when that copy
  returned true and the live file compares equal. On load, before anything reads the file: a good live file wins and
  any `.tmp` is deleted; a bad or missing live file with a whole `.tmp` means an interrupted write, so copy the `.tmp`
  over it; a bad data file with no whole `.tmp` falls back to a good `.bak`. A kill during the copy always leaves a
  whole `.tmp`, which is how an interrupted write differs from a hand edit.
- `CopyFile` with a missing source returns false and leaves the target empty, wiping what was there. Confirm the source
  first (probe, 2026-11-01).
- `FileSerializer.Read` returns true past the end of a file and on an empty one, giving 0, and returns garbage from a
  text file. Never loop on it or trust it to find a cut file; check the file is whole first (probe, 2026-11-01).
- Long strings: past 8,191 characters `Substring` returns at most 8,191 with no error, and `Replace` cuts the string to
  what it found in the first part, emptying it if it found nothing (probes on 492,005 characters, 2026-11-02). A short
  `Substring` from far along works, as do `+`, `==`, `!=`, `Length`, `IndexOf`, `LastIndexOf`, `FPrint` and `ReadFile`.
  Walking lines with `IndexOfFrom` took about 10.9 million `TickCount` ticks for 12,000 rows and `FGets` about 88,000 for
  14,634, so read long files with `FGets`, and join rows in chunks because adding each to the whole copies it every time
  (about 8.7 million ticks for 14,634).
- A `FileSerializer` string stays short: `Write` returns true but cuts it at 8,191 characters, and `Read` returns false
  somewhere between 1,000 and 2,000, with 1,000 fine. Keep strings at 1,000 or under and split anything longer (probe,
  2026-11-01).
- A file parsed by `JsonFileLoader` with no `version` key returns true at version 0 (probe, 2026-11-01).
- A `ref map` field in a `JsonFileLoader` file is replaced whole: a missing key or `null` leaves an empty map, not the
  field's default, so fill defaults after the load. A nested class keeps its defaults for missing keys. Bools save as `0`
  and `1`, both forms load, and a map's keys save in no fixed order (probe, 2026-11-01).
- Dabs' `ProfileSettings` walks every field on its class in `Load` and `Save`, so a field of any other type than `bool`,
  `int`, `float`, `string`, `vector` or `array<string>` hits `Error` each time. Keep anything else on a separate class
  (read in source, 2026-11-02).

### Input, chat and UI

- `KeyState` keeps reporting until `ClearKey`, and `UAUIBack` and the key can fire on different frames, so one Esc can act
  twice if both are read (final review, 2026-11-01; read in source, not seen failing).
- Don't call `ClearKey` on a key that isn't down: first presses vanished until the blind clears went (2026-11-01;
  mechanism inferred from which keys failed).
- `ChatMP` and `g_Game.Chat` use the system channel, which a player's Game messages option hides. The client's `OnChat`
  still fires; a local channel 0 line always shows (probe, 2026-11-01).
- Vanilla's chat box stops taking text when it's visually full (`day_z_chat_input.layout`, `"limit visible" 1`), about 49
  characters at 1920x1080, depending on letter width, and script can't lift that. A longer command arrives cut short and
  reads as a wrong setting or a missing value, so every chat command must fit (client try, 2026-11-02).
- Dabs' text row keeps its start value after Apply, so `IsChanged` stays true and Revert shows the old text while the
  field holds the new one, and `GetProfileString` strips apostrophes on load (`OptionSelectorEditboxView.c:29`,
  `ProfileSettings.c:172`, read, not seen in game). Fix it with a subclass when a real text row needs it.
- Dabs' options menu passes Revert to its custom tabs but not vanilla's Default button (`PerformSetToDefaults`), so
  Default does nothing on a Dabs tab unless a mod handles it (client try, 2026-11-02).
- A widget's alpha is stored in 8 bits: `SetAlpha(0.55)` reads back 0.54902 (140/255). Compare alphas within 0.01, never
  with `==` (client run, 2026-11-06).
- `GetModelOrientation()` on a preview widget answers in the engine's own angle order, not the `<pitch, yaw, roll>` that
  `SetModelOrientation` takes: a pure yaw of 30 reads back `<-30, 0, 0>`, and yaw 30 with pitch 10 reads about `<-30.4,
  8.6, -5.0>`. Vanilla never calls it. Check a turn through a pose you set yourself, or a pure yaw in slot 0 negated
  (client run, 2026-11-06).
- `FindAnyWidget(name)` on a view's root also searches the layouts of views added inside it, depth first: on a window
  root, `"Body"` found a close button's own `Body`, not the window's. A view's bound fields are right; a check that looks
  a node up by name walks the root's direct children, or uses the view's own accessor (client run, 2026-11-06).
- In a check, write each param to a `ScriptReadWriteContext` through a handle of its own type; a base `Param` handle
  writes nothing, so every read fails with no log line and a refusal check passes for the wrong reason (test boot,
  2026-11-07).

### Entities and inventory

- A container sitting in another's cargo takes nothing (`Container_Base.CanReceiveItemIntoCargo`,
  `AreChildrenAccessible`), yet `FindFirstFreeLocationForNewEntity` can offer its cargo, and `CreateInInventory` then
  returns null even if that was the only place found (boot, 2026-11-01). A nested container that works is an attachment.
- A vehicle's `Fill` doesn't change `GetFluidFraction` in the same frame, and the level then arrives over the following
  frames, with or without physics (probe on `OffroadHatchback`: 10 L read 0 at once, about 0.4 L at 0.27 s, all of it at
  2 s; 2026-11-02). `Leak`, `LeakAll` and a later `Fill` changed nothing on a car that had sat for seconds, even grounded
  with physics (same day). A driven car is untested.

### Tooling

- `/usr/bin/mf` can be TeX's METAFONT on a Linux box with TeX installed. ModFoundry's `mf` is normally
  `~/.local/bin/mf`. A script run with a trimmed `PATH` gets METAFONT, which waits at a prompt. Have scripts check
  `mf --version` first (2026-11-04).
