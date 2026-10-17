# Simulates the wasm float_text digit search (src/wasm_emitter/numbers/float_text.rs) in IEEE doubles and compares it with
# Python's repr on random floats: python3 probes/floats/float_scaled_sim.py [count] [values…]
import math, random, struct, sys
SPLITTER = 134217729.0
def bits(x): return struct.unpack('<q', struct.pack('<d', x))[0]
def float_of(b): return struct.unpack('<d', struct.pack('<q', b))[0]
def split(a):
    c = SPLITTER * a; h = c - (c - a); return h, a - h
def exact_product(a, b):
    p = a * b; ah, al = split(a); bh, bl = split(b)
    return p, ((ah * bh - p) + ah * bl + al * bh) + al * bl
def float_scaled(x, gap, shift):
    """(x + gap/2) · 10^shift as hi + lo"""
    scale = 1.0
    if x >= 2.0 ** 900: x *= 2.0 ** -128; gap *= 2.0 ** -128; scale = 2.0 ** 128
    if x < 2.0 ** -900: x *= 2.0 ** 128; gap *= 2.0 ** 128; scale = 2.0 ** -128
    hi, lo = x, gap * 0.5
    while shift:
        step = max(min(shift, 22), -22); power = 10.0 ** abs(step); carried = lo
        if step > 0:
            hi2, err = exact_product(hi, power); lo = err + carried * power; hi = hi2
        else:
            q = hi / power; p, err = exact_product(q, power)
            lo = ((hi - p) - err + carried) / power; hi = q
        s = hi + lo; lo = lo - (s - hi); hi = s
        shift -= step
    return hi * scale, lo * scale
def shortest(x):
    exponent = 0; m = x
    while m >= 10: m /= 10; exponent += 1
    while m < 1: m *= 10; exponent -= 1
    shift = 16 - exponent
    while True:
        hi, lo = float_scaled(x, 0.0, shift)
        if hi > 1e17 or (hi == 1e17 and lo >= 0): shift -= 1; continue
        if hi < 1e16 or (hi == 1e16 and lo < 0): shift += 1; continue
        break
    whole = int(hi) + int(math.floor(lo)); fraction = lo - math.floor(lo)
    b = bits(x); odd = b & 1
    nxt = float_of(b + 1); prev = float_of(b - 1)
    below = x - prev; above = nxt - x if nxt != math.inf else below
    lh, ll = float_scaled(x, -below, shift); hh, hl = float_scaled(x, above, shift)
    low = int(lh) + int(math.ceil(ll)); high = int(hh) + int(math.floor(hl))
    if odd and ll == math.ceil(ll): low += 1
    if odd and hl == math.floor(hl): high -= 1
    p = 10
    while (low + p - 1) // p * p <= high: p *= 10
    p //= 10
    c = whole // p * p; twice = 2 * (whole - c)
    above_half = 2 * fraction + twice - p  # 2 · (v - c) - p, signed: its sign says which multiple is nearer
    if above_half > 0 or (above_half == 0 and c // p % 2 == 1): c += p
    if c < low: c += p
    if c > high: c -= p
    while c % 10 == 0: c //= 10; shift -= 1
    return c, shift
def check(x):
    c, shift = shortest(x)
    digits = repr(x).split('e')[0].replace('.', '').lstrip('0').rstrip('0') if 'e' in repr(x) else repr(x).replace('.', '').strip('0')
    return str(c) == digits.rstrip('0') or str(c).rstrip('0') == digits, c, shift
count = int(sys.argv[1]) if len(sys.argv) > 1 else 100000
bad = 0
for text in sys.argv[2:]:
    print(text, check(float(text)))
random.seed(1)
for i in range(count):
    x = abs(float_of(random.getrandbits(63)))
    if not math.isfinite(x) or x == 0: continue
    ok, c, shift = check(x)
    if not ok:
        bad += 1
        if bad < 10: print('differs', repr(x), c, shift)
for x in [5e-324, 2.2250738585072014e-308, 1.7976931348623157e308, 73/7, 0.1, 0.3, 1e23, 1e22, 6.62607015e-34, 1.5e-30, 9007199254740993.0, 0.1+0.2]:
    ok, c, shift = check(x)
    print(repr(x), ok, c, shift)
print('differs from repr:', bad, 'of', count)
