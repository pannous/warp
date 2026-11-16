"""Eight views of each EZ Tree species, for the forest's far cards.

The view count and the tile size are lib/forest.warp's IMPOSTOR_* constants, so the
sample's tile rects and this image stay one layout. Colour is the near atlas the
mesh shader samples, unlit, with that shader's tints and leaf cut. The normal tile
is the mesh normal in the tree's own axes (the draw rotates it by the tree's yaw).

A view's camera sits on a ring around the trunk. Angle 0 is local +z, and the angle
grows toward local +x, which is impostor_view in lib/forest.warp.
"""
import re
import struct
from pathlib import Path

import numpy as np
from numba import njit
from PIL import Image

# pine, oak, aspen (birch), ash (autumn). Leaf and bark rects are the near atlas.
# Tints, the bark v scale and the leaf cut are ez_forest's shade table.
SPECIES = [
    {"file": "pine_medium", "leaf": (0, 0, 1024, 1024), "bark": (1024, 1024, 256, 512),
     "bark_v": 1.0, "leaf_tint": (1.0, 1.0, 1.0), "bark_tint": (1.0, 1.0, 1.0), "cut": 0.3},
    {"file": "oak_medium", "leaf": (1024, 0, 1024, 1024), "bark": (0, 1024, 256, 512),
     "bark_v": 0.1, "leaf_tint": (0.835, 0.835, 0.804), "bark_tint": (1.0, 0.953, 0.820), "cut": 0.5},
    {"file": "aspen_medium", "leaf": (3072, 0, 1024, 1024), "bark": (512, 1024, 256, 512),
     "bark_v": 1.0, "leaf_tint": (1.0, 0.980, 0.384), "bark_tint": (1.0, 1.0, 1.0), "cut": 0.5},
    {"file": "ash_medium", "leaf": (2048, 0, 1024, 1024), "bark": (0, 1024, 256, 512),
     "bark_v": 0.2, "leaf_tint": (1.0, 1.0, 1.0), "bark_tint": (0.808, 0.800, 0.745), "cut": 0.5},
]
def forest_layout() -> tuple[int, int, int]:
    text = Path(__file__).resolve().parents[2].joinpath("lib/forest.warp").read_text()
    found = {}
    for name in ("IMPOSTOR_VIEWS", "IMPOSTOR_TILE_W", "IMPOSTOR_TILE_H"):
        match = re.search(rf"^{name} = (\d+)\s*$", text, re.M)
        if not match:
            raise SystemExit(f"atlas: {name} is not set in lib/forest.warp")
        found[name] = int(match.group(1))
    return found["IMPOSTOR_VIEWS"], found["IMPOSTOR_TILE_W"], found["IMPOSTOR_TILE_H"]


def load_mesh(folder: Path, stem: str):
    path = folder / "hero" / f"{stem}.eztree"
    normals = folder / "hero" / f"{stem}.nrm"
    if not path.exists() or not normals.exists():
        raise SystemExit(
            f"atlas: no {path.name} beside its normals. "
            "scripts/sample-assets.sh fetch, or node scripts/ez_tree/hero.mjs"
        )
    data = path.read_bytes()
    if data[:4] != b"EZT1":
        raise SystemExit(f"atlas: {path.name} is not an EZT1 mesh")
    levels = struct.unpack_from("<I", data, 4)[0]
    if levels < 1:
        raise SystemExit(f"atlas: {path.name} has no level")
    count_v, count_t = struct.unpack_from("<II", data, 8)
    at = 20
    points = np.frombuffer(data, np.float32, count=count_v * 5, offset=at).reshape(count_v, 5).copy()
    at += count_v * 20
    triangles = np.frombuffer(data, np.uint16, count=count_t * 4, offset=at).reshape(count_t, 4).copy()
    nrm = np.frombuffer(normals.read_bytes(), np.float32, count=count_v * 3).reshape(count_v, 3).copy()
    if len(nrm) != count_v:
        raise SystemExit(f"atlas: {stem}.nrm does not match its mesh")
    radius = float(np.max(np.hypot(points[:, 0], points[:, 2])))
    if not radius > 0:
        raise SystemExit(f"atlas: {stem} has no width")
    return points, triangles, nrm, radius


def axes(theta: float):
    """Camera on the ring at theta (0 is +z, toward +x). right, and the nearness axis."""
    cam = np.array([np.sin(theta), 0.0, np.cos(theta)], np.float64)
    look = -cam
    up = np.array([0.0, 1.0, 0.0], np.float64)
    right = np.cross(look, up)
    right /= np.linalg.norm(right)
    return right, cam


@njit
def _sample(atlas, ox, oy, sx, sy, u, v):
    """sample_rect: fract of the uv, v up, bilinear, clamped to the tile."""
    fu = u - np.floor(u)
    fv = v - np.floor(v)
    px = ox + fu * (sx - 1.0)
    py = oy + (1.0 - fv) * (sy - 1.0)
    x0 = int(np.floor(px))
    y0 = int(np.floor(py))
    if x0 < 0:
        x0 = 0
    if y0 < 0:
        y0 = 0
    if x0 > atlas.shape[1] - 1:
        x0 = atlas.shape[1] - 1
    if y0 > atlas.shape[0] - 1:
        y0 = atlas.shape[0] - 1
    x1 = x0 + 1
    y1 = y0 + 1
    tile_x = int(ox + sx) - 1
    tile_y = int(oy + sy) - 1
    if x1 > tile_x:
        x1 = tile_x
    if y1 > tile_y:
        y1 = tile_y
    if x1 > atlas.shape[1] - 1:
        x1 = atlas.shape[1] - 1
    if y1 > atlas.shape[0] - 1:
        y1 = atlas.shape[0] - 1
    fx = px - np.floor(px)
    fy = py - np.floor(py)
    r0 = atlas[y0, x0, 0] * (1.0 - fx) + atlas[y0, x1, 0] * fx
    r1 = atlas[y1, x0, 0] * (1.0 - fx) + atlas[y1, x1, 0] * fx
    g0 = atlas[y0, x0, 1] * (1.0 - fx) + atlas[y0, x1, 1] * fx
    g1 = atlas[y1, x0, 1] * (1.0 - fx) + atlas[y1, x1, 1] * fx
    b0 = atlas[y0, x0, 2] * (1.0 - fx) + atlas[y0, x1, 2] * fx
    b1 = atlas[y1, x0, 2] * (1.0 - fx) + atlas[y1, x1, 2] * fx
    a0 = atlas[y0, x0, 3] * (1.0 - fx) + atlas[y0, x1, 3] * fx
    a1 = atlas[y1, x0, 3] * (1.0 - fx) + atlas[y1, x1, 3] * fx
    return (
        r0 * (1.0 - fy) + r1 * fy,
        g0 * (1.0 - fy) + g1 * fy,
        b0 * (1.0 - fy) + b1 * fy,
        a0 * (1.0 - fy) + a1 * fy,
    )


@njit
def raster_view(px, py, near, uv, nrm, triangles, atlas, hi_w, hi_h,
                leaf_ox, leaf_oy, leaf_sx, leaf_sy,
                bark_ox, bark_oy, bark_sx, bark_sy,
                bark_v, leaf_tint, bark_tint, cut):
    color = np.zeros((hi_h, hi_w, 4), np.float32)
    normal = np.zeros((hi_h, hi_w, 4), np.float32)
    depth = np.full((hi_h, hi_w), np.float32(-1.0e9))
    for t in range(triangles.shape[0]):
        i0 = triangles[t, 0]
        i1 = triangles[t, 1]
        i2 = triangles[t, 2]
        part = triangles[t, 3]
        x0 = px[i0]
        y0 = py[i0]
        x1 = px[i1]
        y1 = py[i1]
        x2 = px[i2]
        y2 = py[i2]
        min_x = int(np.floor(min(x0, min(x1, x2))))
        max_x = int(np.ceil(max(x0, max(x1, x2))))
        min_y = int(np.floor(min(y0, min(y1, y2))))
        max_y = int(np.ceil(max(y0, max(y1, y2))))
        if min_x < 0:
            min_x = 0
        if min_y < 0:
            min_y = 0
        if max_x >= hi_w:
            max_x = hi_w - 1
        if max_y >= hi_h:
            max_y = hi_h - 1
        if min_x > max_x or min_y > max_y:
            continue
        den = (y1 - y2) * (x0 - x2) + (x2 - x1) * (y0 - y2)
        if abs(den) < 1.0e-8:
            continue
        if part == 0:
            ox, oy, sx, sy = bark_ox, bark_oy, bark_sx, bark_sy
            tint0, tint1, tint2 = bark_tint[0], bark_tint[1], bark_tint[2]
            limit = 0.0
        else:
            ox, oy, sx, sy = leaf_ox, leaf_oy, leaf_sx, leaf_sy
            tint0, tint1, tint2 = leaf_tint[0], leaf_tint[1], leaf_tint[2]
            limit = cut
        for y in range(min_y, max_y + 1):
            for x in range(min_x, max_x + 1):
                xf = x + 0.5
                yf = y + 0.5
                w0 = ((y1 - y2) * (xf - x2) + (x2 - x1) * (yf - y2)) / den
                w1 = ((y2 - y0) * (xf - x2) + (x0 - x2) * (yf - y2)) / den
                w2 = 1.0 - w0 - w1
                if w0 < -1.0e-4 or w1 < -1.0e-4 or w2 < -1.0e-4:
                    continue
                z = w0 * near[i0] + w1 * near[i1] + w2 * near[i2]
                if z <= depth[y, x]:
                    continue
                tu = w0 * uv[i0, 0] + w1 * uv[i1, 0] + w2 * uv[i2, 0]
                tv = w0 * uv[i0, 1] + w1 * uv[i1, 1] + w2 * uv[i2, 1]
                if part == 0:
                    tv *= bark_v
                texel = _sample(atlas, ox, oy, sx, sy, tu, tv)
                if texel[3] < limit:
                    continue
                depth[y, x] = z
                color[y, x, 0] = min(max(texel[0] * tint0, 0.0), 1.0)
                color[y, x, 1] = min(max(texel[1] * tint1, 0.0), 1.0)
                color[y, x, 2] = min(max(texel[2] * tint2, 0.0), 1.0)
                color[y, x, 3] = 1.0
                nx = w0 * nrm[i0, 0] + w1 * nrm[i1, 0] + w2 * nrm[i2, 0]
                ny = w0 * nrm[i0, 1] + w1 * nrm[i1, 1] + w2 * nrm[i2, 1]
                nz = w0 * nrm[i0, 2] + w1 * nrm[i1, 2] + w2 * nrm[i2, 2]
                length = np.sqrt(nx * nx + ny * ny + nz * nz)
                if length < 1.0e-8:
                    length = 1.0
                normal[y, x, 0] = nx / length
                normal[y, x, 1] = ny / length
                normal[y, x, 2] = nz / length
                normal[y, x, 3] = 1.0
    return color, normal


def render_view(points, triangles, nrm, radius, theta, atlas, species, hi_w, hi_h):
    right, cam = axes(theta)
    position = points[:, :3].astype(np.float64)
    horizontal = position @ right
    nearness = position @ cam
    px = (horizontal / radius * 0.5 + 0.5) * (hi_w - 1)
    py = (1.0 - position[:, 1]) * (hi_h - 1)
    leaf = species["leaf"]
    bark = species["bark"]
    return raster_view(
        px, py, nearness, points[:, 3:5].astype(np.float64), nrm.astype(np.float64),
        triangles.astype(np.int32), atlas, hi_w, hi_h,
        float(leaf[0]), float(leaf[1]), float(leaf[2]), float(leaf[3]),
        float(bark[0]), float(bark[1]), float(bark[2]), float(bark[3]),
        float(species["bark_v"]),
        np.array(species["leaf_tint"], np.float64),
        np.array(species["bark_tint"], np.float64),
        float(species["cut"]),
    )


def downsample(color, normal, tile_w, tile_h):
    """2×2 coverage. Alpha is how many samples kept the leaf cut; colour is their mean."""
    c = color.reshape(tile_h, 2, tile_w, 2, 4)
    n = normal.reshape(tile_h, 2, tile_w, 2, 4)
    alpha = c[..., 3]
    covered = alpha.sum(axis=(1, 3))
    rgb = (c[..., :3] * alpha[..., None]).sum(axis=(1, 3))
    rgb /= np.maximum(covered[..., None], 1e-6)
    nrgb = (n[..., :3] * n[..., 3:4]).sum(axis=(1, 3))
    nrgb /= np.maximum(np.linalg.norm(nrgb, axis=-1, keepdims=True), 1e-8)
    out_c = np.zeros((tile_h, tile_w, 4), np.uint8)
    out_n = np.zeros((tile_h, tile_w, 4), np.uint8)
    solid = covered > 0
    out_c[solid, :3] = np.clip(rgb[solid] * 255.0 + 0.5, 0, 255).astype(np.uint8)
    out_c[..., 3] = np.clip(covered / 4.0 * 255.0 + 0.5, 0, 255).astype(np.uint8)
    encoded = np.clip((nrgb * 0.5 + 0.5) * 255.0 + 0.5, 0, 255)
    out_n[solid, :3] = encoded[solid].astype(np.uint8)
    out_n[..., 3] = out_c[..., 3]
    return out_c, out_n


def bake_impostors(folder: Path, near: Image.Image) -> Image.Image:
    views, tile_w, tile_h = forest_layout()
    atlas = np.asarray(near).astype(np.float32) / 255.0
    if atlas.ndim != 3 or atlas.shape[2] != 4:
        raise SystemExit("atlas: the near atlas is not RGBA")
    image = Image.new("RGBA", (views * 2 * tile_w, len(SPECIES) * tile_h), (0, 0, 0, 0))
    hi_w, hi_h = tile_w * 2, tile_h * 2
    for row, species in enumerate(SPECIES):
        points, triangles, nrm, radius = load_mesh(folder, species["file"])
        opaque = 0
        for view in range(views):
            theta = view / views * np.pi * 2.0
            color, normal = render_view(points, triangles, nrm, radius, theta, atlas, species, hi_w, hi_h)
            card, normals = downsample(color, normal, tile_w, tile_h)
            opaque += int((card[..., 3] > 0).sum())
            x = view * 2 * tile_w
            y = row * tile_h
            image.paste(Image.fromarray(card, "RGBA"), (x, y))
            image.paste(Image.fromarray(normals, "RGBA"), (x + tile_w, y))
        print(f"impostor {species['file']} triangles {len(triangles)} radius {radius:.4f} opaque {opaque}", flush=True)
        if opaque < tile_w:
            raise SystemExit(f"atlas: {species['file']} baked an empty impostor")
    return image
