#!/bin/bash
# card g_gGsg: `warp paint-window --check` draws one frame through its wgpu surface (Metal on macOS) and prints the
# center pixel read back from that surface. Opens a window briefly. Usage: probes/paint_window.sh [warp binary]
warp=${1:-data/warp-gv}
expected="center 12 200 99"
# a 64×64 frame, [width u32][height u32] then RGBA bytes, all one color
frame() { python3 -c "import struct,sys; sys.stdout.buffer.write(struct.pack('<II',64,64)+bytes([12,200,99,255])*64*64)"; }
got=$(frame | "$warp" paint-window --check)
[ "$got" = "$expected" ] && echo "ok   $got" || { echo "FAIL got '$got', expected '$expected'"; exit 1; }
