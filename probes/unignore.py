import re, sys
plan = {
 'probe_type': ['test_type_typed_variable'],
 'test_lists': ['test_iteration', 'test_array_indices'],
 'test_math': ['test_eval'],
 'test_samples': ['test_fizzbuzz', 'test_parse_all_samples'],
 'test_todo': ['test_utf8_char_indexing'],
 'test_wasm': ['test_auto_smarty', 'test_math_library', 'test_comparison_primitives'],
}
for file, names in plan.items():
    path = f'tests/{file}.rs'
    lines = open(path).read().split('\n')
    for name in names:
        fn_index = next(i for i, l in enumerate(lines) if re.match(rf'\s*fn {name}\b', l))
        i = fn_index - 1
        while lines[i].strip().startswith('#['):
            if lines[i].strip().startswith('#[ignore'):
                del lines[i]
                break
            i -= 1
        else:
            sys.exit(f'no ignore for {name}')
    open(path, 'w').write('\n'.join(lines))
