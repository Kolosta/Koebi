# 0009. Persistence of the design system state: a versioned binary file keyed by paths

- Status: Accepted
- Date: 2026-10-25
- Deciders: @Kolosta

## Context

The user customizes the design system: switches themes, overrides tokens in one theme or in
all of them, imports themes, exempts tokens from color vision, picks a color vision setting.
All of it must survive restarts on Windows, Linux, macOS, Android and iOS, with these
constraints:

- the token tables and built-in themes are code (ADR 0008): new versions add, rename or remove
  tokens, and saved data must keep working without migration scripts;
- a slider edits a token every frame: saving must not write the disk at that rate;
- a crash or a killed mobile application must never leave a broken file;
- no new dependency (ADR 0003), so no `serde` and no database.

## Decision

`koebi_design::persist` saves only what is not part of the schema: themes created at run time,
the overrides of every theme, global overrides, layer masks that differ from `Layers::ALL`, the
color vision setting and the active theme.

### Format

One binary file, `design.kds`, in the platform's preferences directory
(`AppContext::preferences_dir`, from `SDL_GetPrefPath`):

```text
offset  size  content
     0     8  signature "KOEBIDS\0"
     8     2  format version (1)
    10     2  reserved (0)
    12     4  payload length
    16     4  CRC-32 of the payload
    20     n  payload: schema name, active theme, color vision, run-time themes,
              overrides per theme, global overrides, layer masks
```

- Little-endian numbers; strings are a `u32` length followed by UTF-8; values carry their kind.
- Tokens are referenced by full path (`semantic.background.base.default`) and themes by key,
  never by position: positions change whenever a token is added.
- The same state always encodes to the same bytes (entries sorted by token).

### Restore

The file is parsed in full and checked (signature, version, length, checksum, structure)
before anything is applied. Entries are then applied through the public API, so they get the
same validation as live changes. Whatever no longer fits (unknown token or theme, value out of
range, tier violation, cycle) is skipped and reported, not fatal. Values are applied before
aliases, and aliases in passes until none can be applied any more, so the order of saved
entries never matters.

### Writing

`Store::save` writes `design.kds.tmp`, flushes it to the disk, renames the current file to
`design.kds.bak`, then renames the new file into place. `Store::load` falls back to the backup
when the file is damaged or missing. `Store::save_if_due` saves once the state has stayed
unsaved for a delay (half a second by default): the application calls it every frame, and
calls `Store::save` when it goes to the background or quits.

## Consequences

- Saved data survives schema changes: renamed or removed tokens are reported and dropped on
  the next save, new tokens get their table values.
- A slider drag costs one write per pause; a crash costs at most the last half second.
- The file is not human-readable; the JSON export (issue #44) is the readable form, and the
  binary format stays an implementation detail of the application.
- Format changes bump the version; older builds refuse newer files instead of misreading them.

## Alternatives considered

- **JSON or another text format for the saved state**: readable, but larger and slower to
  parse, and it would duplicate the interchange format with different guarantees. Exports cover
  readability.
- **One file per theme**: several files must then stay consistent with each other; a single
  file is replaced atomically.
- **SQLite or an embedded key-value store**: a dependency, and far more than a few kilobytes of
  overrides need.
- **Positions instead of paths**: smaller, but any change to the token tables would corrupt
  saved data.
- **Saving on every change**: one write per frame while a slider drags.
