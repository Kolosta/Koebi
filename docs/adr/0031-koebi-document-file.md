# 0031. The `.koebi` document: a versioned binary container with its preview, known to each system

- Status: Proposed
- Date: 2026-11-17
- Deciders: @Kolosta
- Settles #18

## Context

Documents need a file that:
- saves the whole graph (ADR 0028) safely;
- opens files from older versions of Koebi, and from newer ones as far as it can;
- never crashes on a damaged file;
- carries a preview of the canvas that the systems' file browsers show.

The maintainer's decisions:
- a binary versioned format, not text;
- the extension `.koebi`, with Koebi's icon;
- a preview built into the file, shown at large icon sizes, of a zone the user can choose, as
  Carto's legacy *Define Thumbnail ▸ Whole Page / Zone…* did;
- extension and preview on every system: on macOS and iOS, the icon only for now, since a
  Quick Look extension needs native code outside Rust;
- no import of Carto's `.acu` files.

What Carto taught:
- Its container (tagged sections, unknown ones skipped) worked.
- Its document payload was positional, version 26 with 25 migrations, and its graph overrides
  were never saved.
- Windows thumbnails came from a COM DLL, registered per user at each launch. But no `open`
  verb was registered and `main()` ignored its arguments: double-clicking a file did not open
  it.

Koebi's design state (ADR 0009) already settled the rules this file follows:
- a signature, a version, a length and a CRC-32;
- references by stable names, never by position;
- the same content always encodes to the same bytes;
- atomic writes with a backup;
- unknown entries skipped and reported.

## Decision

### Container

```text
offset  size  content
     0     8  signature "KOEBI\0\r\n" (catches text-mode transfers)
     8     2  container version (1)
    10     2  flags (0)
    12     4  section count n
    16  32·n  section table, one entry per section:
                tag [u8; 4], flags u32, offset u64, length u64, CRC-32 u32, reserved u32
     …        the sections' payloads, each 8-byte aligned, at the offsets of the table
```

Numbers are little-endian; strings are a `u32` length followed by UTF-8.

- **Sections** are read through the table, so a reader seeks to `PREV` without parsing the
  graph. A thumbnailer reads a few kilobytes.
- **Flags:** a section marked *required* that a reader does not know makes it refuse the file
  with a clear message. Other unknown sections are kept and written back unchanged.
- **Sections of version 1:**
  - `META`: format and application versions, creation and modification dates, document name,
    active module id (ADR 0033), unit system, color mode;
  - `GRPH` (required): the graph;
  - `COLL`: collections;
  - `SWAT`: swatches;
  - `VIEW`: per-document session data worth keeping, such as the last tool per mode and Node
    Graph positions;
  - `PREV`: the preview;
  - `PZON`: the preview zone.

### The graph section

- **Records are keyed, not positional.** A node is its id, its kind's string id, its name, then
  a list of fields `(name, type tag, value)`: input values by socket name, settings by field
  name, Node Graph placement. A link is `(from node, output name, to node, input name, order)`.
- **Old files:** a field a reader does not know is skipped and reported. A field it expects but
  does not find takes its default. Renames and splits are migrations keyed by kind and format
  version, tested on stored sample files.
- **Unknown node kinds** from a newer version become opaque nodes:
  - they keep their fields and their links;
  - they evaluate to nothing;
  - the Node Graph shows them in an error state;
  - they are written back unchanged.
- **Determinism:** nodes are sorted by id, fields by name, and floats are stored as their
  bits. Saving twice gives the same bytes, which the tests check.
- **Geometry** is stored in f64.

### The preview

- **`PREV`:** a PNG, longest side 512 px, sRGB, transparent outside the page if the zone
  exceeds it.
- **How it is made:** on each save, from the document's preview zone, rendered by the canvas
  renderer (ADR 0029) offscreen and read back.
- **The codec:** a std-only crate, `koebi-image`, holds the encoder and decoder that
  `koebi-testkit` has today (with dynamic Huffman blocks for decoding). The canvas will use it
  for images later (#20).
- **`PZON`:** the zone, either *the first page* (default), *a page by id*, or *a rectangle in
  document units*.
  - *Define Preview ▸ Whole Page* comes from a page's context menu in the Viewport and the
    Outliner.
  - *Define Preview ▸ Zone…* comes from the same menus: an interactive rectangle with move and
    resize handles, Enter to confirm and Esc to cancel. It is seeded with Carto's 60 % of the
    page.

### Saving and opening

- **Writing:** `file.koebi.tmp`, flushed, then renamed into place. The previous version is kept
  as `file.koebi.bak` (an option in Settings › General).
- **Opening** parses and checks everything before replacing the open document:
  - signature, versions, table, lengths and checksums;
  - unique ids;
  - links whose ends exist (dangling links are dropped and reported).
- **Errors** name the cause: truncated, damaged section, newer required section, not a Koebi
  file. A damaged file never crashes Koebi.
- **The command line:** `koebi file.koebi` opens the file. `koebi --thumbnail in.koebi out.png
  size` writes the preview and exits, with no window or display (like the exports of #85).

### The systems

- **Windows**, registered per user on each launch (HKCU, no administrator rights; idempotent,
  undone by *Settings › General › Forget file association*):
  - extension `.koebi` → ProgID `Kolosta.Koebi.Document` ("Koebi Document");
  - `DefaultIcon` → a document icon in the executable's resources (Koebi's icon on a page);
  - `shell\open\command` → `"koebi.exe" "%1"`, which Carto lacked;
  - `ShellEx\{e357fccd-a995-4576-b01f-234630154e96}` → the thumbnail provider.
    - It is a new crate, `koebi-thumbnails`: a Windows-only `cdylib` COM in-process server
      implementing `IInitializeWithStream` and `IThumbnailProvider`.
    - It is written with raw FFI in the FFI-glue exception of CLAUDE.md rule 4.
    - It reads `PREV` through the section table, decodes it with `koebi-image`, and returns a
      premultiplied BGRA bitmap.
    - Explorer shows the preview at medium sizes and above, and the icon below.
  - `SHChangeNotify(SHCNE_ASSOCCHANGED)` after any change.
- **Linux:**
  - `platform/linux/koebi.xml`: the shared-mime-info type `application/x-koebi`, glob
    `*.koebi`, magic on the signature;
  - `koebi.thumbnailer`: `Exec=koebi --thumbnail %i %o %s`;
  - the `.desktop` file's `MimeType=`;
  - document icons in the icon theme.

  Packaging installs them. A development build can install them per user with
  `cargo xtask install-desktop`.
- **macOS and iOS:** the bundle's `Info.plist` declares the type, so the system shows Koebi's
  document icon and opens files in Koebi.
  - `UTExportedTypeDeclarations`: `com.kolosta.koebi.document`, conforming to `public.data`,
    extension `koebi`;
  - `CFBundleDocumentTypes` with the icon.
  - No Quick Look preview for now: the maintainer's choice.
- **Android:** an intent filter on the `.koebi` extension and on the MIME type for *open with*
  and the system's document picker, with the app's icon.

## Consequences

- **Compatibility:**
  - Files outlive versions: fields and kinds are named, sections can be skipped, and the
    future can be preserved without being understood.
  - The cost is a little size and parse time over positional records. Numeric data (geometry)
    stays packed, so documents remain compact.
- **Previews:** every saved document carries one. Windows, Linux file managers and Koebi's own
  recent files (splash screen, ADR 0033) use it.
- **New code:**
  - two crates: `koebi-image` (std only, a runtime crate of the workspace like the others) and
    `koebi-thumbnails` (Windows only, FFI);
  - per-system files under `platform/`.
- **Tests:**
  - round trips byte for byte;
  - truncated, damaged and corrupted files at every byte boundary of a small sample;
  - files with unknown sections, fields and kinds, read and written back;
  - old-version samples kept in `crates/koebi-doc/tests/files/`;
  - the thumbnail DLL's decoding, unit-tested on all platforms;
  - its COM surface checked on the Windows CI runner.

## Alternatives considered

- **Text inside a container:** readable and diffable, but larger and slower to load. The
  maintainer chose binary.
- **Positional records (Carto):** compact, but every change is a migration, and an older reader
  cannot keep what it does not know.
- **A zip archive:** a dependency or a zip implementation of our own, for no benefit over a
  section table: Koebi has no separate assets yet. Images can become sections when they come.
- **A Quick Look extension on macOS and iOS:** possible later, in Swift under `platform/`. Left
  out by the maintainer for now.
