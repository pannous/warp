TODO All the suggested optimal styles should be applied with warp beautify / warp fix

if types in seen { return no } => if types in seen : return no 

prefer dot·case over snake_case, kebab-case, CamelCase, context case

def my_fun(): list := [1,2,3]  OK for sort functions BUT:
all_different(variants: list): bool := {...} => def all_different(list variants) as bool {...}   SURE?? no ugly def everywhere
(def my_fun) easier to grep, but LSP can handle?

use "walk.warp" => IDK  import walk  Prefers local files over modules?