# Performance baseline

Taken with `cargo xtask bench` on 2026-11-11 (windows x86_64), CPU Intel64 Family 6 Model 165 Stepping 2, GenuineIntel, GPU NVIDIA GeForce RTX 3060 Laptop GPU (Discrete, Vulkan 1.4.329, driver NVIDIA 596.08).

Wall times, in microseconds, of release builds. Compare them on one machine only. *CPU scene* builds the scene of a screen, as each frame does (warm, 200 frames). *Text layout* lays a sample paragraph out at 600 px. *Offscreen frame* records the frame, waits for the GPU and reads the image back (40 frames, after the two that store glyphs and icons). See issue #30.

| Screen | Measure | Median (µs) | p95 (µs) | Max (µs) | Scene |
| --- | --- | ---: | ---: | ---: | --- |
| Settings › Theme | CPU scene | 534 | 846 | 2892 | 250 rects, 1093 glyphs, 10 icons, 400 draws |
| Settings › Customisation | CPU scene | 166 | 255 | 552 | 16 rects, 221 glyphs, 14 icons, 46 draws |
| Settings › Keymaps | CPU scene | 160 | 285 | 713 | 28 rects, 250 glyphs, 19 icons, 73 draws |
| Settings › Inputs | CPU scene | 290 | 466 | 995 | 78 rects, 572 glyphs, 33 icons, 160 draws |
| Settings › Icons | CPU scene | 170 | 314 | 3766 | 11 rects, 294 glyphs, 122 icons, 57 draws |
| Settings › Language | CPU scene | 249 | 424 | 500 | 14 rects, 311 glyphs, 9 icons, 38 draws |
| Gallery | CPU scene | 1108 | 1548 | 2329 | 274 rects, 1667 glyphs, 24 icons, 354 draws |
| Paragraph (Latin) | text layout | 52 | 71 | 107 |  |
| Paragraph (Japanese) | text layout | 19 | 29 | 101 |  |
| Settings › Theme | offscreen frame | 6453 | 8110 | 9013 |  |
| Settings › Customisation | offscreen frame | 3042 | 3681 | 4018 |  |
| Settings › Keymaps | offscreen frame | 3042 | 3873 | 4331 |  |
| Settings › Inputs | offscreen frame | 3906 | 4807 | 7126 |  |
| Settings › Icons | offscreen frame | 3000 | 3532 | 3776 |  |
| Settings › Language | offscreen frame | 2912 | 3469 | 3917 |  |
| Gallery | offscreen frame | 7631 | 8990 | 9613 |  |

## Start-up

`koebi --frames 1` to its exit, 5 runs: median 1631 ms, fastest 1447 ms (window, Vulkan, pipelines, fonts and the first frame).
