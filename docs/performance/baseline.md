# Performance baseline

Taken with `cargo xtask bench` on 2026-11-11 (windows x86_64), CPU Intel64 Family 6 Model 165 Stepping 2, GenuineIntel, GPU NVIDIA GeForce RTX 3060 Laptop GPU (Discrete, Vulkan 1.4.329, driver NVIDIA 596.08).

Wall times, in microseconds, of release builds. Compare them on one machine only. *CPU scene* builds the scene of a screen, as each frame does (warm, 200 frames). *Text layout* lays a sample paragraph out at 600 px. *Offscreen frame* records the frame, waits for the GPU and reads the image back (40 frames, after the two that store glyphs and icons). See issue #30.

| Screen | Measure | Median (µs) | p95 (µs) | Max (µs) | Scene |
| --- | --- | ---: | ---: | ---: | --- |
| Settings › Theme | CPU scene | 426 | 621 | 1001 | 250 rects, 1033 glyphs, 10 icons, 41 draws |
| Settings › Customisation | CPU scene | 92 | 157 | 421 | 16 rects, 204 glyphs, 14 icons, 18 draws |
| Settings › Keymaps | CPU scene | 78 | 105 | 214 | 28 rects, 238 glyphs, 19 icons, 18 draws |
| Settings › Inputs | CPU scene | 141 | 240 | 505 | 78 rects, 467 glyphs, 33 icons, 18 draws |
| Settings › Icons | CPU scene | 100 | 140 | 314 | 11 rects, 156 glyphs, 122 icons, 18 draws |
| Settings › Language | CPU scene | 100 | 137 | 245 | 14 rects, 281 glyphs, 9 icons, 24 draws |
| Gallery | CPU scene | 512 | 734 | 1714 | 274 rects, 1034 glyphs, 24 icons, 26 draws |
| Gallery, a token dragged | CPU scene | 786 | 1137 | 1667 |  |
| Gallery, the accent dragged | CPU scene | 911 | 1394 | 2469 |  |
| Paragraph (Latin) | text layout | 51 | 82 | 163 |  |
| Paragraph (Japanese) | text layout | 30 | 35 | 244 |  |
| Settings › Theme | offscreen frame | 2738 | 3259 | 3395 |  |
| Settings › Customisation | offscreen frame | 2311 | 2621 | 2710 |  |
| Settings › Keymaps | offscreen frame | 2279 | 2438 | 2671 |  |
| Settings › Inputs | offscreen frame | 2398 | 2592 | 3464 |  |
| Settings › Icons | offscreen frame | 2520 | 2853 | 3114 |  |
| Settings › Language | offscreen frame | 2360 | 2563 | 2905 |  |
| Gallery | offscreen frame | 4656 | 5263 | 5508 |  |

## Start-up

`koebi --frames 1` to its exit, 5 runs: median 2052 ms, fastest 1358 ms (window, Vulkan, pipelines, fonts and the first frame).
