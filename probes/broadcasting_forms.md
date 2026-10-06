# broadcasting and `all` (wiki/broadcasting.md, wiki/all.md): code ||| expected
square(x) := x*x; square [1 2 3] ||| [1 4 9]
square(x) := x*x; xs = [1, 2, 3]; square xs ||| [1 4 9]
square(x) := x*x; square all [1, 2, 3] ||| [1 4 9]
square(x) := x*x; square [a:1 b:2] ||| [a:1 b:4]
upper ["ab", "cd"] ||| ["AB" "CD"]
upper all ["ab", "cd"] ||| ["AB" "CD"]
abs [-1, 2] ||| [1 2]
sqrt [4, 9] ||| [2 3]
f = x => x + 1; f [1, 2] ||| [2 3]
f = x => x + 1; f all [1, 2] ||| [2 3]
add(a, b) := a + b; add [1, 2] 10 ||| [11 12]
add(a, b) := a + b; add(all [1, 2], 10) ||| [11 12]
square(x) := x*x; square [[1, 2], [3]] ||| [[1 4] [9]]
square(x) := x*x; square all [[1, 2], [3]] ||| [[1 4] [9]]
square(x) := x*x; sum square [1, 2, 3] ||| 14
square number = number*number; square [1 2 3] == [1 4 9] ||| 1
