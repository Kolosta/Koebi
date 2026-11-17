# Performance baseline

Taken with `cargo xtask bench` on 2026-11-17 (windows x86_64), CPU Intel64 Family 6 Model 165 Stepping 2, GenuineIntel, GPU NVIDIA GeForce RTX 3060 Laptop GPU (Discrete, Vulkan 1.4.329, driver NVIDIA 596.08).

Wall times, in microseconds, of release builds. Compare them on one machine only. *CPU scene* builds the scene of a screen, as each frame does (warm, 200 frames). *Text layout* lays a sample paragraph out at 600 px. *Offscreen frame* records the frame, waits for the GPU and reads the image back (40 frames, after the two that store glyphs and icons). See issue #30.

| Screen | Measure | Median (µs) | p95 (µs) | Max (µs) | Scene |
| --- | --- | ---: | ---: | ---: | --- |
| Settings › Theme | CPU scene | 660 | 1122 | 1504 | 253 rects, 1013 glyphs, 9 icons, 40 draws |
| Settings › Customisation | CPU scene | 105 | 200 | 307 | 16 rects, 204 glyphs, 14 icons, 18 draws |
| Settings › Keymaps | CPU scene | 79 | 138 | 217 | 28 rects, 238 glyphs, 19 icons, 18 draws |
| Settings › Inputs | CPU scene | 183 | 316 | 498 | 78 rects, 467 glyphs, 33 icons, 18 draws |
| Settings › Icons | CPU scene | 107 | 193 | 314 | 11 rects, 156 glyphs, 122 icons, 18 draws |
| Settings › Language | CPU scene | 81 | 137 | 207 | 14 rects, 281 glyphs, 9 icons, 24 draws |
| Gallery | CPU scene | 447 | 683 | 972 | 272 rects, 1042 glyphs, 24 icons, 26 draws |
| Gallery, a token dragged | CPU scene | 759 | 1230 | 2007 |  |
| Gallery, the accent dragged | CPU scene | 1207 | 1651 | 2103 |  |
| Gallery, the background dragged | CPU scene | 7153 | 8498 | 10290 |  |
| Paragraph (Latin) | text layout | 25 | 40 | 295 |  |
| Paragraph (Japanese) | text layout | 10 | 10 | 451 |  |
| Settings › Theme | offscreen frame | 2708 | 3243 | 3399 |  |
| Settings › Customisation | offscreen frame | 2361 | 2797 | 2939 |  |
| Settings › Keymaps | offscreen frame | 2403 | 2746 | 2885 |  |
| Settings › Inputs | offscreen frame | 2404 | 2813 | 2857 |  |
| Settings › Icons | offscreen frame | 2872 | 4002 | 4444 |  |
| Settings › Language | offscreen frame | 3248 | 4263 | 4465 |  |
| Gallery | offscreen frame | 4858 | 5934 | 6316 |  |

## Geometry

Stroke outlines in `koebi-geom`, within 1/16 of a unit, one stroker kept from stroke to stroke (11 runs after a warm-up). *Smooth strokes* are drawn paths of five cubics; *scribbles* have handles at random, full of loops and near cusps, where the offsets fold; *closed shapes* are ellipses and polygons stroked inside, exactly (the region between two offsets of the shape, by booleans). See issues #258 and #259.

| Geometry | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 smooth strokes | outline, centered, round | 197255 | 207939 | 230261 | 300988 segments |
| 10 000 smooth strokes | outline, centered, miter | 205158 | 231120 | 238792 | 280988 segments |
| 10 000 smooth strokes | outline, right, miter | 129229 | 145412 | 156146 | 243083 segments |
| 10 000 scribbles | outline, centered, round | 506586 | 528211 | 561141 | 1998452 segments |
| 10 000 closed shapes | outline, inside, miter | 1204870 | 1285698 | 1623229 | 135809 segments |
| One long dashed curve | outline, round caps | 131944 | 142511 | 156603 | 177473 long, 17748 dashes, 108884 segments |

## Document

Evaluation of `koebi-doc` into per-page programs (11 runs after a warm-up), on a page of 10 000 rectangles with a fill each: from nothing, then again with one evaluator that keeps its caches. See issue #261.

| Document | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 objects | evaluate from nothing | 133896 | 140769 | 163780 | 10001 steps |
| 10 000 objects | evaluate again, nothing changed | 181 | 221 | 237 |  |
| 10 000 objects | one fill recolored, evaluated again | 11701 | 12349 | 12388 | 4 outputs computed |
| 10 000 objects | one object moved, evaluated again | 10656 | 12593 | 13773 | 3 outputs computed |

## Start-up

`koebi --frames 1` to its exit, 5 runs: median 2093 ms, fastest 1622 ms (window, Vulkan, pipelines, fonts and the first frame).
