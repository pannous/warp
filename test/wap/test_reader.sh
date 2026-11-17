#!/usr/bin/env bash
# wap's reader (wap/core/parse/reader.wap, notes/reader.md): text read as notation, data and code alike, and printed
# back by `wap format` in canonical warp. The cases are notes/reader.md §5 (wasp's tests) and its decisions (§6).
# Run it with bash test/wap/test_reader.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/format_checks.sh"

# atoms: numbers as their value, texts in double quotes, words as written
format_check '1' '1'
format_check '3.' '3.0'
format_check '0xFF' '255'
format_check '1_000 1e3 1.5e-3 3.25' '1000 1000 0.0015 3.25'
format_check "'hi'" '"hi"'
format_check '"\x41\u{1F600}"' '"A😀"'
format_check '`hello $test`' '"hello $test"'
format_check 'yes' 'yes'
format_check 'null' 'ø'
# lists: brackets kept, separators cascade (the loosest splits first), trailing separators drop
format_check '1 2 3' '1 2 3'
format_check '(1,2,3)' '(1, 2, 3)'
format_check '1;2;3' '1; 2; 3'
format_check '[1 2 3]' '[1 2 3]'
format_check '[1,2,]' '[1, 2]'
format_check 'a b; c d' 'a b; c d'
format_check 'a,b,c;d,e,f' 'a, b, c; d, e, f'
format_check '{ a b , c d ; e f , g h }' '{a b, c d; e f, g h}'
# keys: `:` glued binds tight, `: ` takes the rest of the phrase, ` : ` all before it
format_check '{aa:3}' '{aa: 3}'
format_check "a:'HIO' b:3 c:{x:1}" 'a:"HIO" b:3 c: {x: 1}'
format_check "{deep{a:3,b:4,c:{d:'hi'}}}" '{deep{a: 3, b: 4, c: {d: "hi"}}}'
format_check 'a b:c' 'a b: c'
format_check 'a: b c d' 'a: b c d'
format_check 'a b : c' '(a b): c'
format_check 'a-b:2 c-d:4 a-b' '(a - b):2 (c - d):4 a - b'
format_check 'x:40;x+1' 'x: 40; x + 1'
# glue: a head glued to brackets is one item, a blank makes two
format_check 'a{x:1}' 'a{x: 1}'
format_check 'a:{x:1}' 'a: {x: 1}'
format_check 'a b (c)' 'a b (c)'
format_check 'a b(c)' 'a b(c)'
format_check "div{ class:'bold' 'text'}" 'div{class:"bold" "text"}'
format_check "body(style='blue'){a(link)}" 'body(style = "blue"){a(link)}'
format_check '#[0 1 2 3]' '#[0 1 2 3]'
format_check '1 m + 1km' '1 m + 1km'
# signs: glued to digits where an item starts a number, else an operator; operators join repeats and one `=`
format_check '1 - -3' '1 - -3'
format_check 'x=-1' 'x = -1'
format_check 'a=1' 'a = 1'
# indentation: a deeper line opens a block glued to the line before; inside brackets it means nothing
format_check $'a{\n  b,c}' 'a{b, c}'
format_check $'a\n  b\n  c\nd\ne' $'a{b\nc}\nd\ne'
format_check $'a:\n  b\n  c\n\nd\ne' $'a: {b\nc}\nd\ne'
format_check $'a\n  b\n    c\nd' $'a{b{c}}\nd'
format_check $'  a\n    b\n  c' $'a{b}\nc'
# comments are no code
format_check $'blah a b c # note\ny/* c */=0 // c' $'blah a b c\ny = 0'
exit $((failures > 0))
