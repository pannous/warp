#!/usr/bin/env python3
"""Bake the Fish Kit's low red-sea-bream glb into a text mesh.

madai_lo.glb is glTF 2.0 with an EXT_texture_webp atlas and no bones. The
sample cannot decode WebP or bind a texture, so this writes positions,
normals, the kit's part id, fin weight, fold angle and pivot, a colour
sampled from the atlas, and a jaw weight. Pivots are stored in the model's
own axes (the glb stores them swapped; the kit's shader undoes that).

Eyes in the file have a zero pivot. The kit fills each eye with that side's
centre at load, and so does this.

The low mesh has no gill-cover or jaw plate (the kit cuts those from the
head photo at load). Two stand-ins keep the part-id scheme on this mesh:
cheek vertices inside the kit's madai gill box become part 7, and the lower
lip keeps part 0 with a jaw weight, which is the kit's jaw.x on the body.

Usage: scripts/fish-mesh.py <madai_lo.glb> <out.fish>
"""

import json
import struct
import subprocess
import sys
import tempfile

# madai's mouth joint and gill box, metres, from the kit's MOUTHS / GILLS tables.
MOUTH_JOINT_X = 0.19
LIP_X = 0.205
LIP_Y = -0.014
GILL_X = (0.10, 0.16)
GILL_Y = (-0.03, 0.025)
GILL_Z = 0.015
GILL_WEIGHT = 0.012

COMPONENTS = {5120: ("b", 1), 5121: ("B", 1), 5122: ("h", 2), 5123: ("H", 2), 5125: ("I", 4), 5126: ("f", 4)}
WIDTHS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}


def glb(path):
    data = open(path, "rb").read()
    magic, version, length = struct.unpack_from("<III", data, 0)
    if magic != 0x46546C67 or version != 2:
        raise SystemExit(f"{path} is not a glTF 2 binary")
    offset = 12
    document = blob = None
    while offset + 8 <= length:
        size, kind = struct.unpack_from("<II", data, offset)
        offset += 8
        chunk = data[offset:offset + size]
        offset += size
        if kind == 0x4E4F534A:
            document = json.loads(chunk)
        elif kind == 0x004E4942:
            blob = chunk
    if document is None or blob is None:
        raise SystemExit(f"{path} has no JSON and BIN chunks")
    return document, blob


def accessor(document, blob, index):
    acc = document["accessors"][index]
    view = document["bufferViews"][acc["bufferView"]]
    code, size = COMPONENTS[acc["componentType"]]
    width = WIDTHS[acc["type"]]
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    stride = view.get("byteStride", size * width)
    values = []
    for i in range(acc["count"]):
        unpacked = struct.unpack_from("<" + code * width, blob, start + i * stride)
        values.append(unpacked if width > 1 else unpacked[0])
    return values


def atlas(document, blob):
    image = document["images"][0]
    view = document["bufferViews"][image["bufferView"]]
    start = view.get("byteOffset", 0)
    webp = blob[start:start + view["byteLength"]]
    with tempfile.NamedTemporaryFile(suffix=".webp") as image_file:
        image_file.write(webp)
        image_file.flush()
        probe = subprocess.run(
            ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0", image_file.name],
            capture_output=True, check=True,
        )
        decoded = subprocess.run(
            ["ffmpeg", "-v", "error", "-i", image_file.name, "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1"],
            capture_output=True, check=True,
        )
    width, height = (int(part) for part in probe.stdout.decode().strip().split(","))
    pixels = decoded.stdout
    if len(pixels) != width * height * 4:
        raise SystemExit(f"atlas decoded to {len(pixels)} bytes, not {width}×{height}")
    return width, height, pixels


def texel(atlas_image, u, v):
    width, height, pixels = atlas_image
    # glTF v = 0 is the top row. The kit loads the atlas with flipY off, and a
    # side view of this mesh only looks like the fish with that reading.
    x = min(width - 1, max(0, int(round(min(max(u, 0.0), 1.0) * (width - 1)))))
    y = min(height - 1, max(0, int(round(min(max(v, 0.0), 1.0) * (height - 1)))))
    i = (y * width + x) * 4
    return [pixels[i + channel] / 255 for channel in range(4)]


def model_pivot(stored):
    # the kit's shader: piv = (_piv.x, _piv.z, -_piv.y)
    return (stored[0], stored[2], -stored[1])


def num(value, places):
    if abs(value) < 0.5 * 10 ** (-places):
        value = 0.0
    return f"{value:.{places}f}"


def bake(path):
    document, blob = glb(path)
    primitive = document["meshes"][0]["primitives"][0]
    attributes = {name: accessor(document, blob, index) for name, index in primitive["attributes"].items()}
    indices = accessor(document, blob, primitive["indices"])
    image = atlas(document, blob)
    positions = attributes["POSITION"]
    normals = attributes["NORMAL"]
    uvs = attributes["TEXCOORD_0"]
    fins = attributes["_FIN"]
    pivots = attributes["_PIV"]
    swim = document["nodes"][0]["extras"]["swim"]

    count = len(positions)
    eyes = {1: [], -1: []}
    for index, fin in enumerate(fins):
        if int(round(fin[0])) != 6:
            continue
        side = 1 if positions[index][2] >= 0 else -1
        eyes[side].append(index)
    eye_pivot = {}
    for side, indices_on_side in eyes.items():
        if not indices_on_side:
            continue
        centre = [sum(positions[i][axis] for i in indices_on_side) / len(indices_on_side) for axis in range(3)]
        for index in indices_on_side:
            eye_pivot[index] = centre

    lines = [
        "# madai_lo, red sea bream, baked by scripts/fish-mesh.py",
        "# v x y z nx ny nz part weight fold px py pz r g b a jaw",
        "swim "
        + " ".join(
            num(float(value), 5)
            for value in (
                swim["snout"], swim["length"], swim["lambda"], swim["freq"], swim["amp"], *swim["env"],
            )
        ),
    ]
    parts = []
    jaws = []
    for index in range(count):
        fin = fins[index]
        part = int(round(fin[0]))
        weight = float(fin[1])
        fold = float(fin[2])
        pivot = eye_pivot.get(index, model_pivot(pivots[index]))
        position = positions[index]
        jaw = 0.0
        if part == 0 and position[0] > LIP_X and position[1] < LIP_Y:
            jaw = 1.0
        if (
            part == 0
            and GILL_X[0] < position[0] < GILL_X[1]
            and GILL_Y[0] < position[1] < GILL_Y[1]
            and abs(position[2]) > GILL_Z
        ):
            part = 7
            weight = GILL_WEIGHT
            jaw = 0.0
        colour = texel(image, uvs[index][0], uvs[index][1])
        parts.append(part)
        jaws.append(jaw)
        numbers = (
            [num(position[axis], 5) for axis in range(3)]
            + [num(normals[index][axis], 4) for axis in range(3)]
            + [str(part), num(weight, 4), num(fold, 4)]
            + [num(pivot[axis], 5) for axis in range(3)]
            + [num(channel, 4) for channel in colour]
            + [num(jaw, 3)]
        )
        lines.append("v " + " ".join(numbers))
    for triangle in range(0, len(indices), 3):
        lines.append("f " + " ".join(str(indices[triangle + corner] + 1) for corner in range(3)))
    text = "\n".join(lines) + "\n"
    return text, {
        "vertices": count,
        "triangles": len(indices) // 3,
        "parts": {part: parts.count(part) for part in sorted(set(parts))},
        "jaws": sum(1 for weight in jaws if weight > 0.5),
        "bytes": len(text.encode()),
    }


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: scripts/fish-mesh.py <madai_lo.glb> <out.fish>")
    text, summary = bake(sys.argv[1])
    open(sys.argv[2], "w", encoding="utf-8").write(text)
    print(json.dumps(summary, sort_keys=True))


if __name__ == "__main__":
    main()
