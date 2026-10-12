#!/usr/bin/env python3
"""gpu-draw: draw a .warp scene with WARP_DRAW=cpu and =gpu (headless) and compare the PNGs pixel by pixel.
Usage: compare.py <warp binary> <scene.warp> [tolerance per channel, default 1]"""
import os, re, struct, subprocess, sys, zlib

PAETH, CHANNELS = 4, {0: 1, 2: 3}


def painted_png(warp, scene, mode):
    run = subprocess.run([warp, scene], capture_output=True, text=True, env={**os.environ, "WARP_NO_WINDOW": "1", "WARP_DRAW": mode})
    found = re.search(r"painted \d+×\d+: (.+)", run.stderr)
    if not found:
        sys.exit(f"{mode}: nothing painted\n{run.stdout}{run.stderr}")
    return open(found.group(1), "rb").read()


def paeth(left, up, up_left):
    guess = left + up - up_left
    return min((abs(guess - left), left), (abs(guess - up), up), (abs(guess - up_left), up_left))[1]


def pixels(png):
    at, chunks, data = 8, {}, b""
    while at < len(png):
        length, kind = struct.unpack(">I4s", png[at:at + 8])
        body = png[at + 8:at + 8 + length]
        if kind == b"IHDR":
            chunks["size"] = struct.unpack(">II", body[:8]) + (body[9],)
        if kind == b"IDAT":
            data += body
        at += 12 + length
    width, height, color_type = chunks["size"]
    step = CHANNELS[color_type]
    raw, rows, previous = zlib.decompress(data), [], bytearray(width * step)
    for row in range(height):
        start = row * (width * step + 1)
        kind, line = raw[start], bytearray(raw[start + 1:start + 1 + width * step])
        for i in range(len(line)):
            left = line[i - step] if i >= step else 0
            up_left = previous[i - step] if i >= step else 0
            line[i] = (line[i] + [0, left, previous[i], (left + previous[i]) // 2, paeth(left, previous[i], up_left)][kind]) % 256
        rows.append([tuple(line[x * step:(x + 1) * step]) * (3 // step) for x in range(width)])
        previous = line
    return rows


def main():
    warp, scene = sys.argv[1], sys.argv[2]
    tolerance = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    cpu, gpu = (pixels(painted_png(warp, scene, mode)) for mode in ("cpu", "gpu"))
    different = [(x, y, cpu[y][x], gpu[y][x]) for y in range(len(cpu)) for x in range(len(cpu[0])) if max(abs(a - b) for a, b in zip(cpu[y][x], gpu[y][x])) > tolerance]
    print(f"{len(different)} of {len(cpu) * len(cpu[0])} pixels differ by more than {tolerance}")
    for x, y, a, b in different[:20]:
        print(f"  {x},{y}: cpu {a} gpu {b}")
    sys.exit(1 if different else 0)


main()
