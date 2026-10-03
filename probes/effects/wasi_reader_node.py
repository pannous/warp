# Applies the wasm_reader refactor: shared Val→Node conversion, WASI runner returns Node.
import sys
p = sys.argv[1] + '/src/wasm_reader.rs'
s = open(p).read()
start = s.index('\t// Convert result to Node - handle both primitives and GC refs\n\tlet result = &results[0];\n')
end = s.index('/// Create a node by calling a constructor function')
body = s[start:end]
s = s[:start] + '\tval_to_node(&results[0], &mut store, &instance)\n}\n\n' + s[end:]
converter = body.replace('\t// Convert result to Node - handle both primitives and GC refs\n\tlet result = &results[0];\n', '')
converter = '/// Convert a `main` result (primitive or GC Node struct) into a Node, for any store state\nfn val_to_node<T>(result: &Val, mut store: &mut Store<T>, instance: &Instance) -> Result<Node> {\n' + converter
s = s.replace('/// Create a node by calling a constructor function', converter + '\n/// Create a node by calling a constructor function', 1)
s = s.replace('pub fn read_bytes_with_wasi(bytes: &[u8]) -> Result<i64> {', 'pub fn read_bytes_with_wasi(bytes: &[u8]) -> Result<Node> {')
s = s.replace('\tlet mut results = vec![Val::I64(0)];\n\tmain.call(&mut store, &[], &mut results)?;\n\n\tOk(results[0].unwrap_i64())\n', '\tlet mut results = vec![Val::I32(0)];\n\tmain.call(&mut store, &[], &mut results)?;\n\tval_to_node(&results[0], &mut store, &instance)\n')
open(p, 'w').write(s)
