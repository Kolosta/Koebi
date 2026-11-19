# Performance baseline

Taken with `cargo xtask bench` on 2026-11-19 (windows x86_64), CPU Intel64 Family 6 Model 165 Stepping 2, GenuineIntel, GPU NVIDIA GeForce RTX 3060 Laptop GPU (Discrete, Vulkan 1.4.329, driver NVIDIA 596.08).

Wall times, in microseconds, of release builds. Compare them on one machine only. *CPU scene* builds the scene of a screen, as each frame does (warm, 200 frames). *Text layout* lays a sample paragraph out at 600 px. *Offscreen frame* records the frame, waits for the GPU and reads the image back (40 frames, after the two that store glyphs and icons). See issue #30.

| Screen | Measure | Median (µs) | p95 (µs) | Max (µs) | Scene |
| --- | --- | ---: | ---: | ---: | --- |
| Settings › Theme | CPU scene | 614 | 722 | 1359 | 253 rects, 1013 glyphs, 9 icons, 40 draws |
| Settings › Customisation | CPU scene | 113 | 126 | 187 | 16 rects, 204 glyphs, 14 icons, 18 draws |
| Settings › Keymaps | CPU scene | 74 | 125 | 457 | 28 rects, 238 glyphs, 19 icons, 18 draws |
| Settings › Inputs | CPU scene | 255 | 312 | 408 | 78 rects, 467 glyphs, 33 icons, 18 draws |
| Settings › Icons | CPU scene | 99 | 163 | 240 | 11 rects, 156 glyphs, 122 icons, 18 draws |
| Settings › Language | CPU scene | 76 | 92 | 145 | 14 rects, 281 glyphs, 9 icons, 24 draws |
| Gallery | CPU scene | 471 | 574 | 890 | 272 rects, 1042 glyphs, 24 icons, 26 draws |
| Gallery, a token dragged | CPU scene | 749 | 912 | 1522 |  |
| Gallery, the accent dragged | CPU scene | 1154 | 1371 | 1930 |  |
| Gallery, the background dragged | CPU scene | 6561 | 7906 | 9488 |  |
| Paragraph (Latin) | text layout | 22 | 23 | 55 |  |
| Paragraph (Japanese) | text layout | 9 | 9 | 179 |  |
| Settings › Theme | offscreen frame | 2582 | 2888 | 2966 |  |
| Settings › Customisation | offscreen frame | 2188 | 2600 | 2638 |  |
| Settings › Keymaps | offscreen frame | 2309 | 2655 | 3420 |  |
| Settings › Inputs | offscreen frame | 2402 | 2973 | 3112 |  |
| Settings › Icons | offscreen frame | 2458 | 2844 | 3105 |  |
| Settings › Language | offscreen frame | 2314 | 2516 | 2721 |  |
| Gallery | offscreen frame | 4340 | 4752 | 5004 |  |

## Geometry

Stroke outlines in `koebi-geom`, within 1/16 of a unit, one stroker kept from stroke to stroke (11 runs after a warm-up). *Smooth strokes* are drawn paths of five cubics; *scribbles* have handles at random, full of loops and near cusps, where the offsets fold; *closed shapes* are ellipses and polygons stroked inside, exactly (the region between two offsets of the shape, by booleans). See issues #258 and #259.

| Geometry | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 smooth strokes | outline, centered, round | 184168 | 201037 | 206412 | 300988 segments |
| 10 000 smooth strokes | outline, centered, miter | 177568 | 189852 | 203228 | 280988 segments |
| 10 000 smooth strokes | outline, right, miter | 106425 | 117739 | 123007 | 243083 segments |
| 10 000 scribbles | outline, centered, round | 420710 | 446253 | 448314 | 1998452 segments |
| 10 000 closed shapes | outline, inside, miter | 1033270 | 1092467 | 1144776 | 135809 segments |
| One long dashed curve | outline, round caps | 124907 | 142517 | 165047 | 177473 long, 17748 dashes, 108884 segments |

## Document

Evaluation of `koebi-doc` into per-page programs (11 runs after a warm-up), on a page of 10 000 rectangles with a fill each: from nothing, then again with one evaluator that keeps its caches. See issue #261.

| Document | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 objects | evaluate from nothing | 117059 | 140469 | 149184 | 10001 steps |
| 10 000 objects | evaluate again, nothing changed | 0 | 0 | 0 |  |
| 10 000 objects | one fill recolored, evaluated again | 2008 | 2481 | 2901 | 4 outputs computed |
| 10 000 objects | one object moved, evaluated again | 1898 | 2054 | 2124 | 3 outputs computed |

## Canvas

Views of `koebi-canvas` (11 runs after a warm-up), full HD, on the document above, all 10 000 objects in sight; then 500 groups blended in every mode; then 10 000 strokes one pixel wide. Times are the render and the GPU's work, waited for, unless a row says the CPU alone; no validation layers. Preparing outlines (an octave of zoom crossed, strokes in pixels at a new level) runs on the render thread for now. See issues #16 and #19.

| Canvas | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 paths | first render, every cell prepared | 53010 | 58128 | 59146 | 10233 cells |
| 10 000 paths | still view, nothing recorded (CPU) | 0 | 0 | 0 |  |
| 10 000 paths | pan by 7 px, cells kept | 1536 | 1650 | 1688 | 0 outlines prepared |
| 10 000 paths | zoom sweep, 49 steps over 3 octaves and back | 1236 | 9663 | 56723 | 18481 outlines prepared |
| 10 000 paths | one object moved: evaluated and drawn | 4867 | 5560 | 5590 | 12 outlines prepared |
| 10 000 paths | one fill recolored: evaluated and drawn | 5072 | 5648 | 5756 | 0 outlines prepared |
| 10 000 paths | pick under the pointer, 2 px of tolerance (CPU) | 0 | 1 | 25 | 489 hits in 1 000 |
| 500 blended groups | first render, targets made | 29398 | 30146 | 30163 | 1 000 groups, 500 blended in a mode |
| 500 blended groups | pan by 7 px, targets pooled | 15758 | 16085 | 17256 |  |
| 500 blended groups | pan by 7 px, recording alone (CPU) | 4799 | 4922 | 4939 |  |
| 10 000 hairlines | first render, every stroke outlined | 205196 | 224022 | 224556 | 10000 outlined |
| 10 000 hairlines | zoom to the next level and back | 3253 | 3998 | 4630 | 8600 outlined |

## Start-up

`koebi --frames 1` to its exit, 5 runs: median 2106 ms, fastest 1676 ms (window, Vulkan, pipelines, fonts and the first frame).
