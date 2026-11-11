#!/usr/bin/env bash
# The full Japanese Bee, 978285 splats, as a .splat file warp reads (lib/splats.warp).
# Source: "Japanese Bee" by YUMA株式会社, https://superspl.at/scene/ae58ed2c, CC BY 4.0.
# The scene player loads the SOG at the cloudfront URL below (meta.json, count 978285, splat-transform v2.5.2).
# This writes data/splat-speed/bee-full.splat (gitignored, about 31 MB) and does not commit it.
# The 24000-splat samples/graphics/bee.splat is a later cut: opacity below 0.05 dropped, then binned (bee.splat.md).
# This file keeps every Gaussian, in the SOG's order.
#
#   scripts/bee-full.sh
#   warp samples/graphics/gaussian_splats.warp --full
#   warp samples/graphics/gaussian_splats.warp --times data/splat-speed/bee-full.splat
#
# Needs curl, python3 and Node (npx). splat-transform 2.5.2 reads the SOG and writes a binary PLY
# (log scale, spherical-harmonic DC, logit opacity, rot_0 = w). It does not write .splat, so the python
# below writes the 32-byte records lib/splats.warp reads: linear scale, rgb bytes, opacity byte/255,
# quaternion bytes round(q · 128 + 128) in wxyz. That is warp's layout, not splat-transform's .splat reader
# (which uses byte/255 · 2 − 1).
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
scene=https://d28zzqy0iyovbz.cloudfront.net/ae58ed2c/v1
work=$root/data/splat-speed/bee-sog
out=$root/data/splat-speed/bee-full.splat
count=978285
mkdir -p "$work"
echo "downloading $scene/meta.json"
curl -fsSL "$scene/meta.json" -o "$work/meta.json"
python3 - "$work/meta.json" "$count" <<'PY'
import json, sys
meta = json.load(open(sys.argv[1]))
want = int(sys.argv[2])
got = meta["count"]
if got != want:
    raise SystemExit(f"meta.json count is {got}, the scene is {want}")
names = []
for key in ("means", "scales", "quats", "sh0", "shN"):
    part = meta.get(key)
    if isinstance(part, dict):
        names += part.get("files") or []
open(sys.argv[1] + ".files", "w").write("\n".join(names) + "\n")
print(f"{got} splats, {len(names)} files")
PY
while IFS= read -r name; do
	[ -n "$name" ] || continue
	echo "downloading $name"
	curl -fsSL "$scene/$name" -o "$work/$name"
done < "$work/meta.json.files"
ply=$root/data/splat-speed/bee-full.ply
echo "SOG to PLY with @playcanvas/splat-transform@2.5.2"
npx --yes @playcanvas/splat-transform@2.5.2 "$work/meta.json" "$ply"
python3 - "$ply" "$out" "$count" <<'PY'
import math, struct, sys
ply, out, want = sys.argv[1], sys.argv[2], int(sys.argv[3])
SH_C0 = 0.28209479177387814
data = open(ply, "rb").read()
end = data.find(b"end_header\n")
if end < 0:
    raise SystemExit("not a PLY")
header = data[:end].decode("ascii", "replace").splitlines()
if not any(line.startswith("format binary_little_endian") for line in header):
    raise SystemExit("expected a binary little-endian PLY")
count = None
names = []
in_vertex = False
for line in header:
    parts = line.split()
    if len(parts) >= 3 and parts[0] == "element" and parts[1] == "vertex":
        count = int(parts[2])
        in_vertex = True
    elif parts and parts[0] == "element":
        in_vertex = False
    elif in_vertex and len(parts) >= 3 and parts[0] == "property":
        names.append(parts[2])
if count != want:
    raise SystemExit(f"PLY has {count} vertices, the scene has {want}")
need = ["x", "y", "z", "scale_0", "scale_1", "scale_2", "f_dc_0", "f_dc_1", "f_dc_2", "opacity", "rot_0", "rot_1", "rot_2", "rot_3"]
missing = [name for name in need if name not in names]
if missing:
    raise SystemExit("PLY is missing " + ", ".join(missing))
index = {name: i for i, name in enumerate(names)}
body = data[end + len(b"end_header\n"):]
row = 4 * len(names)
if len(body) < count * row:
    raise SystemExit(f"PLY body is {len(body)} bytes, {count} rows of {row} would be {count * row}")
def unit_byte(q):
    return max(0, min(255, int(round(q * 128 + 128))))
def color_byte(dc):
    v = 0.5 + SH_C0 * dc
    return max(0, min(255, int(round(max(0.0, min(1.0, v)) * 255))))
def opacity_byte(logit):
    p = 1.0 / (1.0 + math.exp(-logit)) if logit >= 0 else math.exp(logit) / (1.0 + math.exp(logit))
    return max(0, min(255, int(round(p * 255))))
written = 0
with open(out, "wb") as dst:
    for i in range(count):
        vals = struct.unpack_from("<" + "f" * len(names), body, i * row)
        x, y, z = vals[index["x"]], vals[index["y"]], vals[index["z"]]
        sx, sy, sz = math.exp(vals[index["scale_0"]]), math.exp(vals[index["scale_1"]]), math.exp(vals[index["scale_2"]])
        r, g, b = color_byte(vals[index["f_dc_0"]]), color_byte(vals[index["f_dc_1"]]), color_byte(vals[index["f_dc_2"]])
        a = opacity_byte(vals[index["opacity"]])
        qw, qx, qy, qz = (vals[index[name]] for name in ("rot_0", "rot_1", "rot_2", "rot_3"))
        length = math.sqrt(qw * qw + qx * qx + qy * qy + qz * qz) or 1.0
        qw, qx, qy, qz = qw / length, qx / length, qy / length, qz / length
        dst.write(struct.pack("<ffffff", x, y, z, sx, sy, sz))
        dst.write(bytes((r, g, b, a, unit_byte(qw), unit_byte(qx), unit_byte(qy), unit_byte(qz))))
        written += 1
size = written * 32
print(f"wrote {out}: {written} splats, {size} bytes")
if size != want * 32:
    raise SystemExit(f"size {size} is not {want * 32}")
PY
echo "bee-full.splat is ready: $out"
