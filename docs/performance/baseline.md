# Performance baseline

Taken with `cargo xtask bench` on 2026-11-18 (windows x86_64), CPU Intel64 Family 6 Model 165 Stepping 2, GenuineIntel, GPU NVIDIA GeForce RTX 3060 Laptop GPU (Discrete, Vulkan 1.4.329, driver NVIDIA 596.08).

Wall times, in microseconds, of release builds. Compare them on one machine only. *CPU scene* builds the scene of a screen, as each frame does (warm, 200 frames). *Text layout* lays a sample paragraph out at 600 px. *Offscreen frame* records the frame, waits for the GPU and reads the image back (40 frames, after the two that store glyphs and icons). See issue #30.

| Screen | Measure | Median (µs) | p95 (µs) | Max (µs) | Scene |
| --- | --- | ---: | ---: | ---: | --- |
| Settings › Theme | CPU scene | 613 | 701 | 1031 | 253 rects, 1013 glyphs, 9 icons, 40 draws |
| Settings › Customisation | CPU scene | 114 | 127 | 261 | 16 rects, 204 glyphs, 14 icons, 18 draws |
| Settings › Keymaps | CPU scene | 78 | 112 | 461 | 28 rects, 238 glyphs, 19 icons, 18 draws |
| Settings › Inputs | CPU scene | 243 | 308 | 961 | 78 rects, 467 glyphs, 33 icons, 18 draws |
| Settings › Icons | CPU scene | 98 | 174 | 677 | 11 rects, 156 glyphs, 122 icons, 18 draws |
| Settings › Language | CPU scene | 79 | 95 | 147 | 14 rects, 281 glyphs, 9 icons, 24 draws |
| Gallery | CPU scene | 541 | 690 | 1292 | 272 rects, 1042 glyphs, 24 icons, 26 draws |
| Gallery, a token dragged | CPU scene | 805 | 1090 | 1780 |  |
| Gallery, the accent dragged | CPU scene | 1171 | 1524 | 1878 |  |
| Gallery, the background dragged | CPU scene | 6548 | 8263 | 10322 |  |
| Paragraph (Latin) | text layout | 24 | 27 | 121 |  |
| Paragraph (Japanese) | text layout | 9 | 9 | 183 |  |
| Settings › Theme | offscreen frame | 2520 | 3076 | 3292 |  |
| Settings › Customisation | offscreen frame | 2160 | 2329 | 3013 |  |
| Settings › Keymaps | offscreen frame | 2174 | 2363 | 2517 |  |
| Settings › Inputs | offscreen frame | 2303 | 2446 | 2628 |  |
| Settings › Icons | offscreen frame | 2328 | 2468 | 2654 |  |
| Settings › Language | offscreen frame | 2251 | 2402 | 2533 |  |
| Gallery | offscreen frame | 4409 | 5301 | 5637 |  |

## Geometry

Stroke outlines in `koebi-geom`, within 1/16 of a unit, one stroker kept from stroke to stroke (11 runs after a warm-up). *Smooth strokes* are drawn paths of five cubics; *scribbles* have handles at random, full of loops and near cusps, where the offsets fold; *closed shapes* are ellipses and polygons stroked inside, exactly (the region between two offsets of the shape, by booleans). See issues #258 and #259.

| Geometry | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 smooth strokes | outline, centered, round | 196044 | 202764 | 209047 | 300988 segments |
| 10 000 smooth strokes | outline, centered, miter | 177951 | 197807 | 201648 | 280988 segments |
| 10 000 smooth strokes | outline, right, miter | 106992 | 117364 | 124508 | 243083 segments |
| 10 000 scribbles | outline, centered, round | 428330 | 471240 | 515170 | 1998452 segments |
| 10 000 closed shapes | outline, inside, miter | 1018031 | 1110087 | 1312255 | 135809 segments |
| One long dashed curve | outline, round caps | 117009 | 137428 | 138241 | 177473 long, 17748 dashes, 108884 segments |

## Document

Evaluation of `koebi-doc` into per-page programs (11 runs after a warm-up), on a page of 10 000 rectangles with a fill each: from nothing, then again with one evaluator that keeps its caches. See issue #261.

| Document | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 objects | evaluate from nothing | 115920 | 128874 | 133903 | 10001 steps |
| 10 000 objects | evaluate again, nothing changed | 0 | 0 | 0 |  |
| 10 000 objects | one fill recolored, evaluated again | 2257 | 2462 | 2502 | 4 outputs computed |
| 10 000 objects | one object moved, evaluated again | 2073 | 2656 | 2863 | 3 outputs computed |

## Canvas

Views of `koebi-canvas` (11 runs after a warm-up), full HD, on the document above, all 10 000 objects in sight; then 10 000 strokes one pixel wide. Times are the render and the GPU's work, waited for, unless a row says the CPU alone. Preparing outlines (an octave of zoom crossed, strokes in pixels at a new level) runs on the render thread for now. See issue #16.

| Canvas | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 paths | first render, every cell prepared | 54389 | 58192 | 59677 | 10233 cells |
| 10 000 paths | still view, nothing recorded (CPU) | 0 | 0 | 0 |  |
| 10 000 paths | pan by 7 px, cells kept | 1443 | 1566 | 1795 | 0 outlines prepared |
| 10 000 paths | zoom sweep, 49 steps over 3 octaves and back | 1366 | 9550 | 58494 | 18481 outlines prepared |
| 10 000 paths | one object moved: evaluated and drawn | 6022 | 6207 | 6254 | 12 outlines prepared |
| 10 000 paths | one fill recolored: evaluated and drawn | 5851 | 5966 | 6037 | 0 outlines prepared |
| 10 000 paths | pick under the pointer, 2 px of tolerance (CPU) | 0 | 1 | 10 | 489 hits in 1 000 |
| 10 000 hairlines | first render, every stroke outlined | 195739 | 202318 | 202371 | 10000 outlined |
| 10 000 hairlines | zoom to the next level and back | 3175 | 3702 | 4388 | 8600 outlined |

## Start-up

`koebi --frames 1` to its exit, 5 runs: median 2030 ms, fastest 1676 ms (window, Vulkan, pipelines, fonts and the first frame).
