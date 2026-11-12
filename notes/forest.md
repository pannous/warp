# A million trees

Library `lib/forest.warp`, tests `test/graphics/test_forest.pairs.warp`. Two samples: `samples/graphics/million_trees.warp`,
the billboard forest that runs anywhere (the page too), and `samples/graphics/ez_forest.warp`, the same woods with EZ
Tree meshes near the eye, which needs the large assets in `samples/graphics/ez_trees` that git does not keep (their
NOTICE.md says how to regenerate them; without them `read_tree_mesh` refuses, naming the scripts).
Neither is named `forest.warp`: that name is what `use forest` loads.

Near and mid trees can be the EZ Tree meshes in `lib/meshlets.warp` (`samples/graphics/ez_trees`, four species
in order: pine, oak, birch, autumn). ez_forest's horizon view uses `forest_horizon`. The largest crowns, out to where a tall crown is about
40 px, are the far stride of the medium preset (`*_far.eztree`, one leaf card in eight) while the triangle
budget lasts. A crown that does not fit stays the shaded crown and the dark pine, the same billboard the
earlier forest drew. The meshed trees sample the near atlas (full-size leaves, id 3 in `ez_trees/atlas.bin`). A fresh bin
(zlib RGBA8) is uploaded in one read; a missing or stale stamp rebuilds the `.rgba` beside it
(sources and licences: `ez_trees/NOTICE.md`). `samples/graphics/tree_study.warp` draws the medium presets
full size, the trees the EZ Tree demo opens on.

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
plus one sky quad, which also paints the meadow per pixel where its ray meets the ground (no ground quads:
projected per corner, a quad with a corner behind the eye folds into a wall).

Counters on a view:

- `total` is 1000000
- `candidate` is the trees whose chunks meet the view (every tree of a chunk that is fully inside, and each
  tree that itself meets the view when the chunk is clipped)
- `submitted` is the tree quads in the draw, at most 2400
- `scenery` is the sky quad
- `nearest` is the detail of the closest submitted tree: 0 near, 1 mid, 2 far, or -1 when no tree was drawn

`forest_grove(canopy, glance, kits, screen_height, budget, cap)` takes those four meshes after `pack_tree_meshes`.
The sample passes `MESH_TRIANGLE_BUDGET` and `MESH_TREE_CAP`; a caller passes a smaller pair to draw less.
A tree
whose sprite detail is near or mid picks a level with `chosen_level` (one pixel, `FOREST_PIXEL_ERROR`). The
distance is divided by the tree's scale, because the stored error is in tree heights and the draw is
`height * FOREST_TREE_SCALE` (1.6, the same scale the billboard shader reads as `$tree_scale`). Clusters of
that level are frustum culled in the tree's own frame. Far sprites stay billboards. At most `MESH_TREE_CAP`
(64) trees are meshed, and at most `MESH_TRIANGLE_BUDGET` (150000) triangles. The nearest meshable trees are
taken first. A tree that does not fit stays a billboard, except the first, which may be partial. Draws are
The draw uses a depth buffer. Triangles inside one cluster stay in file order.

ez_forest prints `[total, candidate, submitted, scenery, meshed, triangles]` for the opening view.
`submitted` is still the budget's tree count and includes the trees that were then drawn as meshes.
`meshed` is how many of those became meshes, and `triangles` is the mesh triangles actually drawn.
It paints when a window can open, and also when the program argument is `still`
(`warp samples/graphics/ez_forest.warp still`), which is the headless picture at 240×135.
`still large` is the same view at 1280×720. With no window and no `still`, the frame selects the view
and does not paint: a paint with no GPU faults.
The library's opening view (x 80, z -12, yaw 0.15, pitch -0.18, aspect 240/135, the full million,
`forest_view`) is `[1000000, 5797, 2400, 1]`: the trees, plus the sky.
The sample stands at x 2000, y 21.7, z -22, yaw 0, pitch -0.18, on a 1000-tree square, and draws with
`forest_horizon` out to 2000 m. `still` prints `[1000000, 211623, 4963, 1, 0, 0]`.
`still large` prints `[1000000, 211623, 11577, 1, 87, 149936]`.
Drag looks around. W, A, S, D and the arrows walk. Headless with `still`, one frame is painted and the program
value is that list.

## GPU

ez_forest uses `paint` of a shader with its own vertex stage, which draws `instances` quads
(`lib/host/native_host/gpu.warp`, the existing instanced-quad path). The first `sprite_count` quads are the
sprites that stayed billboards: the sky (which paints the meadow per pixel too) or a far tree. Each mesh cluster is then `MESHLET_TRIANGLES`
quads, one a triangle and the last corner a repeat so the strip's second triangle has no area. The other storage
arrays are the places, the triangle spans, the packed vertices, the texels (v and the vertex normal), and the triangles: six, which is
the host's limit. The fragment draws a billboard tree lit by a low sun in the billboard's frame:
tiered ragged pines, oak and autumn crowns of seven clumps (backlit rims when the sun is behind), airy birches with
scarred white bark, each tinted by a hash of its position; the sky quad paints sky, sun, clouds and, below the horizon,
the meadow per pixel; everything hazes with distance. A mesh triangle samples the species bark or leaf from the near atlas. Leaves use a four-tap footprint
so a small card does not sparkle, and the lighting is the study's hemisphere plus the forest sun, from the
vertex normal (a coarse level with no stored normal uses the face). The sample starts at SIDE = 1000. The paint asks for a depth target. Sprites are drawn near-first so a farther
fragment fails the depth test. Farther than the meshed band, a tree is the clumped crown or the tiered pine.

A million trees are not uploaded. The GPU sees one view's sprites and the clusters that view meshed.
