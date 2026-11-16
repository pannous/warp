# Fish

A swimming red sea bream, `samples/gpu/fish.warp`. One photo mesh, posed on the CPU. `paint` still has no vertex stage, and a texture binding is still `gpu-multipass`, so the fragment shader ray-tests the posed triangles. The loader, the pose and that ray live in `samples/gpu/fish_mesh.warp`, which the aquarium instances too (`notes/aquarium.md`). No compiler or core change.

## The mesh

`samples/gpu/madai.fish` is `madai_lo.glb` from なちゃっと's Fish Kit (CC0), baked by `scripts/fish-mesh.py`. The glb is 190 KB because of its WebP atlas. The text mesh is the geometry plus a colour per vertex, about 60 KB. `samples/gpu/madai.NOTICE` records the author, the URLs and the CC0 terms.

The file has no skeleton. The kit's `_FIN` part ids are the motion:

| id | part |
| --- | --- |
| 0 | body, a travelling wave |
| 1, 2 | dorsal and anal fins, folded about the pivot by the stored angle |
| 3 | tail, closed a little when the wave is small |
| 4 | pectoral fins, flapping about the pivot |
| 5 | pelvic fins |
| 6 | eyes, glancing about the pivot |
| 7 | gill cover, hinged outward |
| 8, 9 | lower and upper jaw |

This low mesh only carries parts 0, 1, 4 and 6. The kit cuts gill and jaw plates from the head photo at load; they are not in the file. The baker fills each eye's zero pivot with that side's centre (what the kit does at load), hinges the cheek vertices inside the kit's gill box as part 7, and writes a jaw weight on the lower lip (the kit's `jaw.x` on the body). Parts 2, 3, 5 and 9 are in the poser for the scheme. Part 1 here has no pivot and a zero fold, so those fins ride the body wave, which is what swings the tail.

Regenerate with the glb in hand:

```bash
scripts/fish-mesh.py madai_lo.glb samples/gpu/madai.fish
```

The atlas is inside the glb. ffmpeg reads the WebP. v = 0 is the top row, which is the reading that looks like the fish.

## The picture

`fish_point(mesh, index, seconds)` is one posed vertex. `fish_pose` drops triangles whose average alpha is below one half (the kit's cutout). The frame poses that mesh once, `fish_show` hands the corners to the shader, and `fish_nearest` ray-tests them. The cruise is the kit's steady swim for this species: 0.4 m/s, the model's own 1.6 Hz, the envelope from the glb. The window orbits. Headless, `on frame` paints one still.

```bash
warp samples/gpu/fish.warp
```

`test/fish/test_fish.warp` parses a tiny mesh and one deformed vertex of each motion, and the baked mesh's counts, tail swing, lip, pectoral, eye and gill. It does not claim a GPU frame was shown.
