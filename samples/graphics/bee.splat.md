# bee.splat

A Gaussian-splatting capture of a Japanese honeybee (Apis cerana japonica), read by gaussian_splats.warp: 24000
splats in the .splat layout lib/splats.warp reads (32 bytes each, 768000 bytes).

## Source

Japanese Bee by YUMA株式会社 (SuperSplat user yyouzhen).

- Scene: https://superspl.at/scene/ae58ed2c
- Author: YUMA株式会社 — https://superspl.at/user/yyouzhen
- Licence: Creative Commons Attribution 4.0 International (CC BY 4.0)
  https://creativecommons.org/licenses/by/4.0/

Read on that scene page: the page shows "CC BY 4.0" with the title Attribution, its HTML has
`<link rel="license" href="https://creativecommons.org/licenses/by/4.0/">`, and the published scene record has
`"license": "by"`. The page names the author YUMA株式会社. The capture is a focus-stacked photogrammetry scan
(Sony macro, Agisoft Metashape, Lichtfeld Studio), published as SOG (splat-transform v2.5.2): 978285 splats,
11482650 bytes, spherical harmonics of degree 3.

Attribution: "Japanese Bee" by YUMA株式会社, https://superspl.at/scene/ae58ed2c, CC BY 4.0.

## Changes

Decoded from that SOG (PlayCanvas SOG v2: positions, scales, quaternions, DC color and opacity) into the .splat
layout. The higher-order spherical harmonics are not kept: .splat stores one color. Splats with opacity below
0.05 were dropped (928043 remained). Those were binned in a 40×40×40 grid over the box of their 1st–99th
percentile positions (13430 cells held one). Each cell kept its two highest scores of opacity × geometric-mean
scale, and of that set the 24000 highest scores were written, largest first.
