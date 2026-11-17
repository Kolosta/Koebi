# Performance baseline

Taken with `cargo xtask bench` on 2026-11-17 (windows x86_64), CPU Intel64 Family 6 Model 165 Stepping 2, GenuineIntel, GPU NVIDIA GeForce RTX 3060 Laptop GPU (Discrete, Vulkan 1.4.329, driver NVIDIA 596.08).

Wall times, in microseconds, of release builds. Compare them on one machine only. *CPU scene* builds the scene of a screen, as each frame does (warm, 200 frames). *Text layout* lays a sample paragraph out at 600 px. *Offscreen frame* records the frame, waits for the GPU and reads the image back (40 frames, after the two that store glyphs and icons). See issue #30.

| Screen | Measure | Median (µs) | p95 (µs) | Max (µs) | Scene |
| --- | --- | ---: | ---: | ---: | --- |
| Settings › Theme | CPU scene | 759 | 1959 | 3342 | 253 rects, 1013 glyphs, 9 icons, 40 draws |
| Settings › Customisation | CPU scene | 141 | 330 | 426 | 16 rects, 204 glyphs, 14 icons, 18 draws |
| Settings › Keymaps | CPU scene | 90 | 161 | 347 | 28 rects, 238 glyphs, 19 icons, 18 draws |
| Settings › Inputs | CPU scene | 341 | 578 | 1691 | 78 rects, 467 glyphs, 33 icons, 18 draws |
| Settings › Icons | CPU scene | 134 | 262 | 333 | 11 rects, 156 glyphs, 122 icons, 18 draws |
| Settings › Language | CPU scene | 99 | 176 | 295 | 14 rects, 281 glyphs, 9 icons, 24 draws |
| Gallery | CPU scene | 635 | 1172 | 1678 | 272 rects, 1042 glyphs, 24 icons, 26 draws |
| Gallery, a token dragged | CPU scene | 883 | 1419 | 2018 |  |
| Gallery, the accent dragged | CPU scene | 1336 | 2046 | 3515 |  |
| Gallery, the background dragged | CPU scene | 7123 | 8348 | 9417 |  |
| Paragraph (Latin) | text layout | 23 | 33 | 56 |  |
| Paragraph (Japanese) | text layout | 9 | 9 | 208 |  |
| Settings › Theme | offscreen frame | 2755 | 3774 | 5697 |  |
| Settings › Customisation | offscreen frame | 2248 | 2575 | 3148 |  |
| Settings › Keymaps | offscreen frame | 2328 | 2913 | 4779 |  |
| Settings › Inputs | offscreen frame | 2346 | 2932 | 3186 |  |
| Settings › Icons | offscreen frame | 2423 | 3139 | 4745 |  |
| Settings › Language | offscreen frame | 2334 | 3402 | 4262 |  |
| Gallery | offscreen frame | 4480 | 5814 | 7098 |  |

## Geometry

Stroke outlines in `koebi-geom`, within 1/16 of a unit, one stroker kept from stroke to stroke (11 runs after a warm-up). *Smooth strokes* are drawn paths of five cubics; *scribbles* have handles at random, full of loops and near cusps, where the offsets fold. See issue #258.

| Geometry | Measure | Median (µs) | p95 (µs) | Max (µs) | Output |
| --- | --- | ---: | ---: | ---: | --- |
| 10 000 smooth strokes | outline, centered, round | 192322 | 217633 | 220048 | 300988 segments |
| 10 000 smooth strokes | outline, centered, miter | 194951 | 201494 | 218668 | 280988 segments |
| 10 000 smooth strokes | outline, right, miter | 111469 | 116368 | 117369 | 243083 segments |
| 10 000 scribbles | outline, centered, round | 448076 | 489202 | 489570 | 1998452 segments |
| 10 000 closed shapes | outline, inside, miter | 226341 | 238587 | 239627 | 176624 segments |
| One long dashed curve | outline, round caps | 135561 | 139532 | 156904 | 177473 long, 17748 dashes, 108884 segments |

## Start-up

`koebi --frames 1` to its exit, 5 runs: median 2096 ms, fastest 2075 ms (window, Vulkan, pipelines, fonts and the first frame).
