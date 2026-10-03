import re, subprocess, collections
failing = collections.defaultdict(list); passing_vacuous = []
current = None
for line in open('data/ignored_run.log', errors='ignore'):
    m = re.search(r'Running (\S+)', line)
    if m: current = m.group(1)
    m = re.match(r'test (\S+) \.\.\. FAILED$', line.strip())
    if m: failing[current].append(m.group(1))
def reason(path, name):
    if not path.endswith('.rs'): return 'unit test'
    lines = open(path).read().split('\n')
    i = next((k for k, l in enumerate(lines) if re.match(rf'\s*(pub )?fn {name}\b', l)), None)
    if i is None: return '?'
    while i > 0 and lines[i-1].strip().startswith('#['):
        i -= 1
        m = re.match(r'#\[ignore\s*=\s*"(.*)"\]', lines[i].strip())
        if m: return m.group(1)
        m = re.search(r'#\[ignore\].*//\s*(.*)', lines[i].strip())
        if m: return m.group(1)
    return 'no reason given'
unignored = {'probe_type': ['test_type_typed_variable'], 'test_lists': ['test_iteration', 'test_array_indices'],
 'test_math': ['test_eval'], 'test_samples': ['test_fizzbuzz', 'test_parse_all_samples'],
 'test_todo': ['test_utf8_char_indexing'], 'test_wasm': ['test_auto_smarty', 'test_math_library', 'test_comparison_primitives']}
for file, names in unignored.items():
    path = f'tests/{file}.rs'
    still = [f'  {n}: {reason(path, n)}' for n in failing.get(path, [])]
    body = f"test: un-ignore {len(names)} already-passing test(s) in {file}\n\nUn-ignored (pass under --ignored, only the attribute removed):\n" + \
        "".join(f"  {n}\n" for n in names) + \
        ("\nStill failing in this file, left ignored:\n" + "\n".join(still) + "\n" if still else "\nNo other ignored test in this file fails.\n")
    subprocess.run(['git', 'add', path], check=True)
    print(subprocess.run(['git', 'diff', '--cached', '--stat'], capture_output=True, text=True).stdout)
    subprocess.run(['git', 'commit', '-q', '-m', body], check=True)
    r = subprocess.run(['git', 'push'], capture_output=True, text=True); print(r.stdout[-200:], r.stderr[-200:])
