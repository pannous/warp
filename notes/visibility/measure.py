#!/usr/bin/env python3
"""Library word visibility (card library-word-visibility): which std-module functions are untyped (no parameter type,
no return type, `any` counting as none), and which test/sample programs call one of a module they use directly."""
import re, sys, pathlib, collections
ROOT = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else '.')
HEAD = re.compile(r'^(?:def\s+|fun\s+|public\s+|export\s+)*([A-Za-z_][\w]*)\s*\(([^)]*)\)\s*(:\s*[^=]+?)?\s*(:=|\{|=>)')
def untyped(params, result):
    if result and result.strip(': ').strip() not in ('', 'any'): return False
    for p in [p for p in params.split(',') if p.strip()]:
        if ':' in p and p.split(':', 1)[1].split('=')[0].strip() not in ('', 'any'): return False
    return True
modules = {}
for f in sorted((ROOT / 'lib').glob('*.warp')):
    words = {}
    for line in f.read_text(errors='ignore').splitlines():
        m = HEAD.match(line)
        if m and not line.startswith('public'):
            name, params, result = m.group(1), m.group(2), m.group(3)
            if name in ('if', 'for', 'while', 'class', 'match'): continue
            typed = not untyped(params, result)
            words[name] = words.get(name, False) or typed
    modules[f.stem] = words
private = {m: {w for w, typed in ws.items() if not typed} for m, ws in modules.items()}
total = sum(len(ws) for ws in modules.values()); priv = sum(len(p) for p in private.values())
print(f'{priv} of {total} std-module functions untyped (private unless public)')
USE = re.compile(r'\buse\s+([a-z_]\w*)')
OWN = re.compile(r'^\s*(?:def\s+|public\s+)?([A-Za-z_]\w*)\s*\([^)]*\)[^=]*:=', re.M)
hits = collections.Counter(); where = collections.defaultdict(set)
callers = [f for d in ('test', 'samples', 'lib', 'web', 'examples') if (ROOT / d).exists() for f in (ROOT / d).rglob('*.warp')]
callers = [f for f in callers if 'compiler' not in f.parts and 'warp.wasm' not in str(f)]
def scan(f, lines, uses, own):
    for n, line in lines:
        for used in uses(line):
            if f.parent == ROOT / 'lib' and f.stem == used: continue
            for w in private.get(used, ()):
                if w not in own(line) and re.search(r'(?<![\w.])' + re.escape(w) + r'\s*\(', line):
                    hits[(used, w)] += 1; where[(used, w)].add(f'{f.relative_to(ROOT)}:{n}')
for f in callers:
    text = f.read_text(errors='ignore'); lines = list(enumerate(text.splitlines(), 1))
    if f.name.endswith('.pairs.warp'):
        scan(f, lines, USE.findall, lambda line: set(OWN.findall(line)) | {m.group(1) for m in re.finditer(r'\b([A-Za-z_]\w*)\s*\([^)]*\)\s*:=', line)})
    else:
        file_uses = set(USE.findall(text)); file_own = set(OWN.findall(text))
        if f.parent == ROOT / 'lib': file_own |= set(modules.get(f.stem, {}))
        scan(f, lines, lambda line: file_uses, lambda line: file_own)
print(f'{len(hits)} private words called by {len({x for s in where.values() for x in s})} lines')
for (m, w), c in sorted(hits.items(), key=lambda kv: (kv[0][0], kv[0][1])):
    print(f'{m}.{w}\t{c}\t' + ' '.join(sorted(where[(m, w)])[:3]))
