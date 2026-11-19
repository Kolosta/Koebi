# Performance baseline

Taken with `cargo xtask bench` on 2026-11-19 (windows x86_64), CPU Intel64 Family 6 Model 165 Stepping 2, GenuineIntel, GPU NVIDIA GeForce RTX 3060 Laptop GPU (Discrete, Vulkan 1.4.329, driver NVIDIA 596.08).

Wall times, in microseconds, of release builds. Compare them on one machine only. *CPU scene* builds the scene of a screen, as each frame does (warm, 200 frames). *Text layout* lays a sample paragraph out at 600 px. *Offscreen frame* records the frame, waits for the GPU and reads the image back (40 frames, after the two that store glyphs and icons). See issue #30.

| Screen | Measure | Median (µs) | p95 (µs) | Max (µs) | Scene |
| --- | --- | ---: | ---: | ---: | --- |
| Settings › Theme | CPU scene | 654 | 1058 | 1260 | 253 rects, 1013 glyphs, 9 icons, 40 draws |
| Settings › Customisation | CPU scene | 118 | 161 | 386 | 16 rects, 204 glyphs, 14 icons, 18 draws |
| Settings › Keymaps | CPU scene | 84 | 154 | 548 | 28 rects, 238 glyphs, 19 icons, 18 draws |
| Settings › Inputs | CPU scene | 264 | 423 | 858 | 78 rects, 467 glyphs, 33 icons, 18 draws |
| Settings › Icons | CPU scene | 113 | 164 | 366 | 11 rects, 156 glyphs, 122 icons, 18 draws |
| Settings › Language | CPU scene | 82 | 100 | 256 | 14 rects, 281 glyphs, 9 icons, 24 draws |
| Gallery | CPU scene | 534 | 708 | 1297 | 272 rects, 1042 glyphs, 24 icons, 26 draws |
| Gallery, a token dragged | CPU scene | 835 | 1095 | 1489 |  |
| Gallery, the accent dragged | CPU scene | 1221 | 1769 | 3260 |  |
| Gallery, the background dragged | CPU scene | 7256 | 9446 | 15617 |  |
| Paragraph (Latin) | text layout | 27 | 27 | 49 |  |
| Paragraph (Japanese) | text layout | 10 | 18 | 105 |  |
| Settings › Theme | offscreen frame | 2566 | 2903 | 3387 |  |
| Settings › Customisation | offscreen frame | 2208 | 2407 | 2831 |  |
| Settings › Keymaps | offscreen frame | 2208 | 2408 | 2517 |  |
| Settings › Inputs | offscreen frame | 2288 | 2580 | 2877 |  |
| Settings › Icons | offscreen frame | 2363 | 2613 | 3140 |  |
| Settings › Language | offscreen frame | 2285 | 4079 | 35233 |  |
| Gallery | offscreen frame | 4152 | 4672 | 4817 |  |

## Geometry

Stroke outlines in `koebi-geom`, within 1/16 of a unit, one stroker kept from stroke to stroke (11 runs after a warm-up). *Smooth strokes* are drawn paths of five cubics; *scribbles* have handles at random, full of loops and near cusps, where the offsets fold; *closed shapes* are ellipses and polygons stroked inside, exactly (the region between two offsets of the shape, by booleans). See issues #258 and #259.

| Geometry | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 smooth strokes | outline, centered, round | 190483 | 212914 | 220424 | 300988 segments |
| 10 000 smooth strokes | outline, centered, miter | 190086 | 202036 | 218315 | 280988 segments |
| 10 000 smooth strokes | outline, right, miter | 110416 | 127629 | 128294 | 243083 segments |
| 10 000 scribbles | outline, centered, round | 448278 | 469368 | 484380 | 1998452 segments |
| 10 000 closed shapes | outline, inside, miter | 1138977 | 1407251 | 1417185 | 135809 segments |
| One long dashed curve | outline, round caps | 121265 | 127408 | 136206 | 177473 long, 17748 dashes, 108884 segments |

## Document

Evaluation of `koebi-doc` into per-page programs (11 runs after a warm-up), on a page of 10 000 rectangles with a fill each: from nothing, then again with one evaluator that keeps its caches. See issue #261.

| Document | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 objects | evaluate from nothing | 119508 | 138008 | 146479 | 10001 steps |
| 10 000 objects | evaluate again, nothing changed | 0 | 0 | 0 |  |
| 10 000 objects | one fill recolored, evaluated again | 2130 | 2317 | 2372 | 4 outputs computed |
| 10 000 objects | one object moved, evaluated again | 1918 | 2171 | 2304 | 3 outputs computed |

## Canvas

Views of `koebi-canvas` (11 runs after a warm-up), full HD, on the document above, all 10 000 objects in sight; then 500 groups blended in every mode; then 10 000 strokes one pixel wide. Times are the render and the GPU's work, waited for, unless a row says the CPU alone; no validation layers. Preparing outlines (an octave of zoom crossed, strokes in pixels at a new level) runs on the render thread for now. See issues #16 and #19.

| Canvas | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 paths | first render, every cell prepared | 49012 | 53268 | 55225 | 10233 cells |
| 10 000 paths | still view, nothing recorded (CPU) | 0 | 0 | 0 |  |
| 10 000 paths | pan by 7 px, cells kept | 1529 | 1716 | 1731 | 0 outlines prepared |
| 10 000 paths | zoom sweep, 49 steps over 3 octaves and back | 1281 | 9900 | 55623 | 18481 outlines prepared |
| 10 000 paths | one object moved: evaluated and drawn | 5108 | 5867 | 6548 | 12 outlines prepared |
| 10 000 paths | one fill recolored: evaluated and drawn | 5034 | 5674 | 6774 | 0 outlines prepared |
| 10 000 paths | pick under the pointer, 2 px of tolerance (CPU) | 0 | 1 | 11 | 489 hits in 1 000 |
| 500 blended groups | first render, targets made | 26974 | 28588 | 32016 | 1 000 groups, 500 blended in a mode |
| 500 blended groups | pan by 7 px, targets pooled | 15498 | 15964 | 16083 |  |
| 500 blended groups | pan by 7 px, recording alone (CPU) | 4802 | 5185 | 7542 |  |
| 10 000 hairlines | first render, every stroke outlined | 210767 | 239055 | 240629 | 10000 outlined |
| 10 000 hairlines | zoom to the next level and back | 4403 | 4840 | 4848 | 8600 outlined |

## Start-up

`koebi --frames 1` to its exit, 5 runs: median 2104 ms, fastest 2078 ms (window, Vulkan, pipelines, fonts and the first frame).
