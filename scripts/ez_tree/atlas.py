#!/usr/bin/env python3
"""RGBA atlases for the EZ Tree samples.

Reads the textures named in samples/graphics/ez_trees/atlas.sources and writes
hero/ash.rgba, hero/grove.rgba, and atlas.bin beside them. The shader's tile
table is this layout; change both together. atlas.bin is the same pixels,
zlib-compressed RGBA8, so a run can upload them without rebuilding the floats.
Entry 4 is the impostor sheet (scripts/ez_tree/impostor.py). Entries 1–3 keep
the compressed payloads already in atlas.bin when the source hash still matches,
so a new impostor does not rewrite their pixels.

  python3 scripts/ez_tree/atlas.py samples/graphics/ez_trees
"""
import hashlib
import struct
import sys
import zlib
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
from impostor import bake_impostors

LEAF = 512
NEAR_W = 4096
NEAR_H = 1536
GROVE_ID = 1
ASH_ID = 2
NEAR_ID = 3
IMPOSTOR_ID = 4


def leaf_tile(path: Path, size: int) -> Image.Image:
    """Leaf card. A native-size image is copied. A smaller one is a premultiplied
    Lanczos resize, so the alpha stays soft instead of a hard cut."""
    src = Image.open(path).convert("RGBA")
    if src.width == size and src.height == size:
        return src
    arr = np.asarray(src).astype(np.float32) / 255.0
    rgb = arr[..., :3] * arr[..., 3:4]
    premul = Image.fromarray((np.clip(np.dstack([rgb, arr[..., 3]]), 0, 1) * 255).astype(np.uint8), "RGBA")
    small = np.asarray(premul.resize((size, size), Image.Resampling.LANCZOS)).astype(np.float32) / 255.0
    alpha = small[..., 3:4]
    color = np.where(alpha > 0, np.clip(small[..., :3] / np.maximum(alpha, 1e-4), 0, 1), 0)
    return Image.fromarray((np.clip(np.dstack([color, small[..., 3]]), 0, 1) * 255).astype(np.uint8), "RGBA")


def bark_tile(path: Path) -> Image.Image:
    return Image.open(path).convert("RGB").resize((256, 512), Image.Resampling.LANCZOS)


# atlas.bin: magic, version, count, sha256 of the source list, then one entry
# per image (id, width, height, format, raw size, payload size, file offset)
# and the zlib payloads. Format 1 is RGBA8. The stamp in atlas.txt repeats the
# hash and the sizes so a run can check them without reading the payload.
MAGIC = b"EZAT"
VERSION = 1
RGBA8 = 1
HEADER = 4 + 4 + 4 + 32
ENTRY = 7 * 4


def source_names(root: Path) -> list[str]:
    names = []
    for line in (root / "atlas.sources").read_text().splitlines():
        name = line.strip()
        if name and not name.startswith("#"):
            names.append(name)
    return names


def source_hash(root: Path, names: list[str]) -> str:
    parts = [hashlib.sha256((root / name).read_bytes()).hexdigest() for name in names]
    return hashlib.sha256("\n".join(parts).encode()).hexdigest()


def kept_payloads(path: Path, digest: str, images: list[tuple[int, int, int, bytes]]) -> list[bytes] | None:
    """The compressed bytes of entries 1–3, when this bin is the same bake of them."""
    if not path.exists():
        return None
    data = path.read_bytes()
    if len(data) < HEADER or data[:4] != MAGIC or struct.unpack_from("<I", data, 4)[0] != VERSION:
        return None
    count = struct.unpack_from("<I", data, 8)[0]
    if count < 3 or len(data) < HEADER + count * ENTRY:
        return None
    if data[12:44] != bytes.fromhex(digest):
        return None
    out = []
    for index, (ident, width, height, raw) in enumerate(images[:3]):
        base = HEADER + index * ENTRY
        got = struct.unpack_from("<7I", data, base)
        if got[0] != ident or got[1] != width or got[2] != height or got[3] != RGBA8 or got[4] != len(raw):
            return None
        start, size = got[6], got[5]
        if start + size > len(data):
            return None
        out.append(data[start:start + size])
    return out


def pack_atlas(images: list[tuple[int, int, int, bytes]], digest: str, payloads: list[bytes] | None = None) -> bytes:
    if payloads is None:
        payloads = [zlib.compress(raw, 9) for _, _, _, raw in images]
    if len(payloads) != len(images):
        raise SystemExit("atlas: a payload per image")
    directory = HEADER + len(images) * ENTRY
    entries = bytearray()
    blob = bytearray()
    at = directory
    for (ident, width, height, raw), payload in zip(images, payloads):
        entries += struct.pack("<IIIIIII", ident, width, height, RGBA8, len(raw), len(payload), at)
        blob += payload
        at += len(payload)
    return b"".join([
        MAGIC,
        struct.pack("<II", VERSION, len(images)),
        bytes.fromhex(digest),
        entries,
        blob,
    ])


def main() -> None:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "samples/graphics/ez_trees")
    textures = root / "textures"
    hero = root / "hero"
    hero.mkdir(parents=True, exist_ok=True)

    ash = Image.new("RGBA", (1280, 1024), (0, 0, 0, 0))
    ash.paste(leaf_tile(textures / "leaves" / "ash.png", 1024), (0, 0))
    ash.paste(bark_tile(textures / "bark" / "Bark001_1K-JPG" / "Bark001_1K-JPG_Color.jpg"), (1024, 0))
    ash.paste(bark_tile(textures / "bark" / "Bark001_1K-JPG" / "Bark001_1K-JPG_NormalGL.jpg"), (1024, 512))
    (hero / "ash.rgba").write_bytes(ash.tobytes())

    grove = Image.new("RGBA", (2048, 1024), (0, 0, 0, 0))
    for i, name in enumerate(["pine", "oak", "ash", "aspen"]):
        grove.paste(leaf_tile(textures / "leaves" / f"{name}.png", LEAF), (i * LEAF, 0))
    for i, folder in enumerate(["Bark001_1K-JPG", "Bark002_1K-JPG", "Bark003_1K-JPG"]):
        grove.paste(bark_tile(textures / "bark" / folder / f"{folder}_Color.jpg"), (i * 512, 512))
        grove.paste(bark_tile(textures / "bark" / folder / f"{folder}_NormalGL.jpg"), (i * 512 + 256, 512))
    (hero / "grove.rgba").write_bytes(grove.tobytes())

    # Full 1024 leaves, the cards tree_study samples. Bark row under them:
    # Bark001, Bark002, Bark003 colour then each normal. Species order on the
    # leaf row is pine, oak, ash, aspen.
    near = Image.new("RGBA", (NEAR_W, NEAR_H), (0, 0, 0, 0))
    for i, name in enumerate(["pine", "oak", "ash", "aspen"]):
        near.paste(leaf_tile(textures / "leaves" / f"{name}.png", 1024), (i * 1024, 0))
    for i, folder in enumerate(["Bark001_1K-JPG", "Bark002_1K-JPG", "Bark003_1K-JPG"]):
        near.paste(bark_tile(textures / "bark" / folder / f"{folder}_Color.jpg"), (i * 512, 1024))
        near.paste(bark_tile(textures / "bark" / folder / f"{folder}_NormalGL.jpg"), (i * 512 + 256, 1024))
    (hero / "near.rgba").write_bytes(near.tobytes())

    names = source_names(root)
    digest = source_hash(root, names)
    images = [
        (GROVE_ID, grove.size[0], grove.size[1], grove.tobytes()),
        (ASH_ID, ash.size[0], ash.size[1], ash.tobytes()),
        (NEAR_ID, near.size[0], near.size[1], near.tobytes()),
    ]
    kept = kept_payloads(root / "atlas.bin", digest, images)
    # The impostor samples the near atlas the shader will upload. When entries 1–3
    # stay the bytes already on disk, that is the decompressed entry, not a fresh
    # resize that might differ by a texel.
    baked_near = near
    if kept is not None:
        baked_near = Image.frombytes("RGBA", near.size, zlib.decompress(kept[2]))
    impostor = bake_impostors(root, baked_near)
    images.append((IMPOSTOR_ID, impostor.size[0], impostor.size[1], impostor.tobytes()))
    payloads = None
    if kept is not None:
        payloads = kept + [zlib.compress(images[3][3], 9)]
    packed = pack_atlas(images, digest, payloads)
    (root / "atlas.bin").write_bytes(packed)
    fields = ["EZAT", str(VERSION), digest]
    for ident, width, height, _raw in images:
        fields += [str(ident), str(width), str(height)]
    (root / "atlas.txt").write_text(" ".join(fields) + "\n")
    same = "entries 1-3 unchanged" if kept is not None else "entries 1-3 rebuilt"
    print(
        f"ash.rgba {ash.size[0]}x{ash.size[1]}  grove.rgba {grove.size[0]}x{grove.size[1]}"
        f"  near.rgba {near.size[0]}x{near.size[1]}"
        f"  impostor {impostor.size[0]}x{impostor.size[1]}  atlas.bin {len(packed)}  {digest}  {same}"
    )


if __name__ == "__main__":
    main()
