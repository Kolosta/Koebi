# Performance baseline

Taken with `cargo xtask bench` on 2026-11-17 (windows x86_64), CPU Intel64 Family 6 Model 165 Stepping 2, GenuineIntel, GPU NVIDIA GeForce RTX 3060 Laptop GPU (Discrete, Vulkan 1.4.329, driver NVIDIA 596.08).

Wall times, in microseconds, of release builds. Compare them on one machine only. *CPU scene* builds the scene of a screen, as each frame does (warm, 200 frames). *Text layout* lays a sample paragraph out at 600 px. *Offscreen frame* records the frame, waits for the GPU and reads the image back (40 frames, after the two that store glyphs and icons). See issue #30.

| Screen | Measure | Median (µs) | p95 (µs) | Max (µs) | Scene |
| --- | --- | ---: | ---: | ---: | --- |
| Settings › Theme | CPU scene | 646 | 884 | 1193 | 253 rects, 1013 glyphs, 9 icons, 40 draws |
| Settings › Customisation | CPU scene | 116 | 195 | 230 | 16 rects, 204 glyphs, 14 icons, 18 draws |
| Settings › Keymaps | CPU scene | 82 | 111 | 473 | 28 rects, 238 glyphs, 19 icons, 18 draws |
| Settings › Inputs | CPU scene | 252 | 397 | 675 | 78 rects, 467 glyphs, 33 icons, 18 draws |
| Settings › Icons | CPU scene | 110 | 130 | 286 | 11 rects, 156 glyphs, 122 icons, 18 draws |
| Settings › Language | CPU scene | 87 | 127 | 228 | 14 rects, 281 glyphs, 9 icons, 24 draws |
| Gallery | CPU scene | 526 | 649 | 797 | 272 rects, 1042 glyphs, 24 icons, 26 draws |
| Gallery, a token dragged | CPU scene | 774 | 1006 | 1484 |  |
| Gallery, the accent dragged | CPU scene | 1184 | 1638 | 2785 |  |
| Gallery, the background dragged | CPU scene | 6893 | 10386 | 14060 |  |
| Paragraph (Latin) | text layout | 23 | 24 | 54 |  |
| Paragraph (Japanese) | text layout | 9 | 9 | 200 |  |
| Settings › Theme | offscreen frame | 2628 | 2941 | 3381 |  |
| Settings › Customisation | offscreen frame | 2278 | 2464 | 2543 |  |
| Settings › Keymaps | offscreen frame | 2358 | 2636 | 2790 |  |
| Settings › Inputs | offscreen frame | 2628 | 3040 | 3123 |  |
| Settings › Icons | offscreen frame | 2333 | 2461 | 3145 |  |
| Settings › Language | offscreen frame | 2278 | 2443 | 2642 |  |
| Gallery | offscreen frame | 4920 | 6086 | 6308 |  |

## Geometry

Stroke outlines in `koebi-geom`, within 1/16 of a unit, one stroker kept from stroke to stroke (11 runs after a warm-up). *Smooth strokes* are drawn paths of five cubics; *scribbles* have handles at random, full of loops and near cusps, where the offsets fold; *closed shapes* are ellipses and polygons stroked inside, exactly (the region between two offsets of the shape, by booleans). See issues #258 and #259.

| Geometry | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 smooth strokes | outline, centered, round | 194163 | 203198 | 229446 | 300988 segments |
| 10 000 smooth strokes | outline, centered, miter | 196008 | 200511 | 229162 | 280988 segments |
| 10 000 smooth strokes | outline, right, miter | 113004 | 148765 | 182637 | 243083 segments |
| 10 000 scribbles | outline, centered, round | 441075 | 467318 | 506832 | 1998452 segments |
| 10 000 closed shapes | outline, inside, miter | 1053094 | 1102341 | 1115864 | 135809 segments |
| One long dashed curve | outline, round caps | 122925 | 137023 | 161145 | 177473 long, 17748 dashes, 108884 segments |

## Start-up

`koebi --frames 1` to its exit, 5 runs: median 2119 ms, fastest 2090 ms (window, Vulkan, pipelines, fonts and the first frame).
