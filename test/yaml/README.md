# yaml-test-suite subset for lib/yaml (not part of smoke)

`yaml-test-suite/` is a subset of the data branch of https://github.com/yaml/yaml-test-suite at commit
6ad3d2c62885d82fc349026c136ef560838fdf3d (2022-01-17): per case in.yaml, in.json, === (its title) and tags, 140
cases, 560 files, 22 KB (MIT).

Selection rule: valid cases (no `error` file) that have an in.json, are tagged with at least one of sequence mapping
flow scalar anchor alias comment document simple spec literal folded double single indent whitespace empty header
footer, and with none of complex-key explicit-key tag local-tag unknown-tag directive 1.3-err 1.3-mod libyaml-err
error duplicate-key empty-key edge upto-1.2; and whose in.yaml has no line starting with % ? or !, no " !" tag and no
tab character (lib/yaml refuses tags, complex keys, directives and tab indentation by design).

    WARP_LIB=$PWD/lib test/yaml/yaml_test.py [--fails TAG|all]
