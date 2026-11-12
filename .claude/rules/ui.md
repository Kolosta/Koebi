---
paths:
  - "crates/koebi-ui/**"
  - "apps/koebi/src/**"
---

# Interface components (koebi-ui, the application)

- Settle, then draw: handle presses, keys and drags first, then draw every part of the
  component from the settled state, in the same frame.
- Never draw a part inside a lazy branch: no widget call inside `or_else`, `&&`, `||`, or after
  an early `return` or `?` taken because something else changed. Call every part, then combine
  the results eagerly (`let picked = swatches(ui); changed.or(picked)`).
- Test an interaction by comparing what a frame draws during it with a frame at rest.
- Values come from component tokens through `Styles`; texts through `koebi_i18n`.
- Text measured or drawn every frame goes through the line cache (`Ui::text_width`,
  `Ui::label`): never shape the same line twice a frame.
