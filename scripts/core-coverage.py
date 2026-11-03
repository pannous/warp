#!/usr/bin/env python3
"""scripts/core-coverage.py instrument <warp.wasm> <out.wasm>: a copy of a compiler whose every function says on stderr,
the first time it runs, `cov <index>` (its index among the module's defined functions, names in <out.wasm>.names).
scripts/core-coverage.py report <out.wasm> <run output> <tree>: the core's top-level definitions that run never
reached, by file, with their line counts (card core-extensions-roadmap: what the compiler does not need to compile
itself). Measurement only: nothing in the build uses it."""
import re
import subprocess
import sys
from pathlib import Path

FUNCTION_HEADER = re.compile(r'^  \(func \$("(?:[^"\\]|\\.)*"|\S+)')
SAVED_BYTES = 32  # memory 0's first bytes, borrowed for fd_write's iovec and the line, put back after
COVERAGE_MEMORY = '  (memory $__covmem 2)'  # a byte per point, then the saved bytes
COVERAGE_FUNCTION = f'''  (func $__cov (param $id i32) (local $i i32) (local $p i32) (local $n i32)
    (if (i32.load8_u $__covmem (local.get $id)) (then (return)))
    (i32.store8 $__covmem (local.get $id) (i32.const 1))
    (local.set $i (i32.const 0))
    (loop $save
      (i32.store8 $__covmem offset=131000 (local.get $i) (i32.load8_u (local.get $i)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $save (i32.lt_u (local.get $i) (i32.const {SAVED_BYTES}))))
    (i32.store8 (i32.const 31) (i32.const 10))
    (local.set $p (i32.const 31))
    (local.set $n (local.get $id))
    (loop $digit
      (local.set $p (i32.sub (local.get $p) (i32.const 1)))
      (i32.store8 (local.get $p) (i32.add (i32.const 48) (i32.rem_u (local.get $n) (i32.const 10))))
      (local.set $n (i32.div_u (local.get $n) (i32.const 10)))
      (br_if $digit (local.get $n)))
    (local.set $p (i32.sub (local.get $p) (i32.const 4)))
    (i32.store (local.get $p) (i32.const 0x20766f63))
    (i32.store (i32.const 0) (local.get $p))
    (i32.store (i32.const 4) (i32.sub (i32.const 32) (local.get $p)))
    (drop (call $fd_write (i32.const 2) (i32.const 0) (i32.const 1) (i32.const 8)))
    (local.set $i (i32.const 0))
    (loop $restore
      (i32.store8 (local.get $i) (i32.load8_u $__covmem offset=131000 (local.get $i)))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $restore (i32.lt_u (local.get $i) (i32.const {SAVED_BYTES})))))
'''


def function_name(raw: str) -> str:
    if raw.startswith('"'):
        raw = re.sub(r'\\u\{([0-9a-f]+)\}', lambda m: chr(int(m.group(1), 16)), raw[1:-1])
    return raw


BRANCH_START = re.compile(r'^\s+(if|else|block|loop)\b')


def instrument(wasm: str, out: str):
    """a call of $__cov at each function's start and each branch's: points[id] = [function, kind, wat lines up to
    the next point], the lines standing in for the code each point stands for"""
    lines = subprocess.run(['wasm-tools', 'print', wasm], capture_output=True, text=True, check=True).stdout.split('\n')
    result, points = [], []
    index = 0
    function = None
    while index < len(lines):
        line = lines[index]
        result.append(line)
        index += 1
        if points and function:
            points[-1][2] += 1
        if line.startswith('  (memory (;0;)'):
            result.append(COVERAGE_MEMORY)
        header = FUNCTION_HEADER.match(line)
        if header:
            function = function_name(header.group(1))
            while lines[index].startswith('    (local '):
                result.append(lines[index])
                index += 1
            kind = 'entry'
        elif function and BRANCH_START.match(line):
            kind = 'branch'
        else:
            if line.startswith('  (') and not header:
                function = None
            continue
        result.append(f'    i32.const {len(points)}\n    call $__cov')
        points.append([function, kind, 0])
    closing = len(result) - 1 - next(i for i, l in enumerate(reversed(result)) if l.strip() == ')')
    result.insert(closing, COVERAGE_FUNCTION)
    wat = Path(out).with_suffix('.wat')
    wat.write_text('\n'.join(result))
    subprocess.run(['wasm-tools', 'parse', str(wat), '-o', out], check=True)
    Path(out + '.points').write_text(''.join(f'{f}\t{k}\t{w}\n' for f, k, w in points))


TOP_LEVEL_DEFINITION = re.compile(r'^(?:def |fun |global )?([A-Za-z_][A-Za-z_0-9]*)\s*(?:\(|:=|=|:)')


def definitions(file: Path):
    """[name, first line, line count] of each top-level definition, its comments above it counted with it"""
    lines = file.read_text().split('\n')
    found, comments = [], 0
    for number, line in enumerate(lines, 1):
        if line.startswith('#') or line.startswith('//'):
            comments += 1
            continue
        match = TOP_LEVEL_DEFINITION.match(line)
        if match:
            found.append([match.group(1), number - comments, 0])
        comments = 0
    for i, definition in enumerate(found):
        end = found[i + 1][1] if i + 1 < len(found) else len(lines) + 1
        definition[2] = end - definition[1]
    return found


def report(out: str, run_output: str, tree: str):
    """per core file: definitions never run (whole), and the cold share of the ones that ran (their branches never
    taken, by the wat lines they stand for, scaled to the definition's source lines)"""
    points = [line.split('\t') for line in Path(out + '.points').read_text().splitlines()]
    hit = {int(m.group(1)) for m in re.finditer(r'^cov (\d+)$', Path(run_output).read_text(), re.M)}
    total, cold = {}, {}
    for id, (function, kind, weight) in enumerate(points):
        base = function.split('·')[0]
        total[base] = total.get(base, 0) + int(weight)
        if id not in hit:
            cold[base] = cold.get(base, 0) + int(weight)
    root = Path(tree)
    sums = [0, 0, 0]
    for file in sorted((root / 'core').rglob('*.warp')):
        if '/native/' in str(file):
            continue
        whole, partly = [], []
        for name, line, count in definitions(file):
            if name not in total or total[name] == 0:
                continue
            share = cold.get(name, 0) / total[name]
            if share > 0.999:
                whole.append((name, line, count))
            elif share * count >= 3:
                partly.append((name, line, round(share * count)))
        lines = sum(d[2] for d in definitions(file))
        whole_lines, cold_lines = sum(d[2] for d in whole), sum(d[2] for d in partly)
        sums = [sums[0] + lines, sums[1] + whole_lines, sums[2] + cold_lines]
        if whole or partly:
            print(f'{file.relative_to(root)}  {lines} lines, unreached {whole_lines}, cold inside {cold_lines}')
            if whole:
                print('  unreached: ' + ', '.join(f'{n}:{l}({c})' for n, l, c in whole))
            if partly:
                print('  cold inside: ' + ', '.join(f'{n}:{l}(~{c})' for n, l, c in sorted(partly, key=lambda d: -d[2])))
    reached = sum(1 for id in hit if id < len(points) and points[id][1] == 'entry')
    print(f'TOTAL definitions {sums[0]} lines: unreached {sums[1]}, cold inside reached ones ~{sums[2]}; '
          f'functions reached {reached} of {sum(1 for p in points if p[1] == "entry")}, points {len(hit)} of {len(points)}')


if __name__ == '__main__':
    {'instrument': instrument, 'report': report}[sys.argv[1]](*sys.argv[2:])
