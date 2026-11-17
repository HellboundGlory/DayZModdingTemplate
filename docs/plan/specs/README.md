# Job specs

One file per job, small enough for one sitting. A spec holds:

- **Goal**: one or two sentences.
- **Decisions cited**: the decision ids it rests on.
- **Files owned**: the only files the job changes.
- **Contract**: for a new or changed public API, its signatures, errors and call order.
- **Tests first**: the failing tests to write, then the code that makes them pass.
- **Must stay**: behaviour that must not change, and how that is proved.
- **Done when**: the checks that pass and what is left for an in-game check.
