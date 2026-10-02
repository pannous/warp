# selects this fix's hunks from the shared working tree's diff (other agents edit the same files)
import re, sys
d = open(sys.argv[1]).read()
markers = ['ELSE_IF', 'else_if', 'RETURN_KEYWORD', 'try_parse_return', 'UNINDEXABLE', 'unindexable', 'parse_loop_variable', 'PROPERTYLESS',
           'lower_declarations_among', 'lower(', 'counting_phrase', 'property_of_name', "Python's `else", 'names_a_variable', 'IN_KEYWORD', 'counts_units', 'pub(crate) fn text_unit']
foreign = ['try_parse_array_type_suffix']
out = []
for f in re.split(r'(?=^diff --git)', d, flags=re.M)[1:]:
    parts = re.split(r'(?=^@@)', f, flags=re.M)
    out.append(parts[0])
    for h in parts[1:]:
        lines = h.splitlines(True)
        if not any(m in l for l in lines if l[:1] in '+-' for m in markers):
            continue
        while (start := next((i for i, l in enumerate(lines) if l.startswith('+') and any(fm in l for fm in foreign)), None)) is not None:
            del lines[start:start + 3]  # the foreign `if let … { return …; }` block
        out.append(''.join(lines))
open(sys.argv[2], 'w').write(''.join(out))
