# fluid_25d

The picture — a torch whose flame reads as a lit, self-shadowed height, and a pale fog that flows around a pillar — follows a visual idea by [@r4nM4_64](https://x.com/r4nM4_64/status/2120431026191344067), including the Zelda: Ocarina of Time fog study that post quotes. No code was taken from those posts or from any shader they link.

The motion is Stam's stable fluids (Jos Stam, "Stable Fluids", SIGGRAPH 1999), written here in WGSL: an impulse, semi-Lagrangian advection, buoyancy, vorticity confinement, a pressure solve, then a gradient subtracted from the velocity. The shading is a short height-field lighting pass on the density.
