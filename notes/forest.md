# A million trees

Library `lib/forest.warp`, sample `samples/graphics/million_trees.warp`, tests `test/graphics/test_forest.pairs.warp`.
The sample is not named `forest.warp`: that name is what `use forest` loads.

This does not depend on `lib/meshlets.warp`. It does not change the GPU host. It is not Nanite and not a
meshlet renderer: there is no cluster hierarchy, no indexed triangle mesh, and no indirect draw.

## Scene

`forest_view(glance, side)` takes the trees along each edge (a multiple of 25; default 1000, the sample uses 100 = 10,000 trees).
The default is exactly 1,000,000 trees, a 1000 by 1000 grid of 4 metre cells. A tree's position, height and species are a
mix of its cell, so the same index is the same tree and nothing stores a million objects. Species are pine,
oak, birch and autumn. Heights run from 2.4 to just over 5.

Chunks are 25 by 25 trees, 100 metres on a side, 40 by 40 of them. A chunk is a candidate when its box meets
the view and its closest point is not past the far distance. The box is padded by 3 metres so a crown on the
edge is not dropped with its chunk. A box that only touches a plane stays.

## What is drawn

`TREE_BUDGET` is 2400 tree quads. Near chunks (closer than 56 metres) and mid chunks (closer than 180) submit
their trees closest to the eye first, until the budget is spent. Farther chunks submit 4 of their own trees,
spread across the cells: real placements, not one picture standing in for the chunk. The draw is those quads
plus one sky quad and at most 12 ground tiles under the nearest chunks.

Counters on a view:

- `total` is 1000000
- `candidate` is the trees whose chunks meet the view (every tree of a chunk that is fully inside, and each
  tree that itself meets the view when the chunk is clipped)
- `submitted` is the tree quads in the draw, at most 2400
- `scenery` is the sky and ground quads
- `nearest` is the detail of the closest submitted tree: 0 near, 1 mid, 2 far, or -1 when no tree was drawn

The sample prints `[total, candidate, submitted, scenery]` for the opening view, and again each frame.
It paints only when a window can open. With no GPU, `paint` of any shader faults, so a headless run
selects the view and does not draw it.
That opening view (x 80, z -12, yaw 0.15, pitch -0.18, aspect 240/135) is
`[1000000, 5797, 2400, 13]`: 5797 trees meet the view, 2400 of them are drawn, plus the sky and 12 ground tiles.
Drag looks around. W, A, S, D and the arrows walk. Headless, one frame is painted and the program value is
that same list for the opening camera (aspect 240 by 135, the headless frame).

## GPU

The sample uses `paint` of a shader with its own vertex stage, which draws `instances` quads from one storage
array (`lib/host/native_host/gpu.warp`, the existing instanced-quad path). Each quad is one sprite: sky,
a ground tile, or a tree billboard. The fragment draws each tree lit by a low sun in the billboard's frame:
tiered ragged pines, oak and autumn crowns of seven clumps (backlit rims when the sun is behind), airy birches with
scarred white bark, each tinted by a hash of its position; the sky quad paints sky, sun, clouds and, below the horizon,
the meadow per pixel; everything hazes with distance. The sample starts at SIDE = 100 (10,000 trees). There is no depth buffer; far sprites are
submitted before near ones and blended over.

Still absent, and not added here: indexed meshes, indirect draws, a depth attachment, mesh shaders, and
streaming. A million trees are not uploaded. The GPU sees only the sprites of one view.
