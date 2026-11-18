#!/usr/bin/env bash
# wap's dynamic typing (notes/first_principles.md: "dynamic where we can't infer the type", "any is the dynamic
# type, and inference falls back to it", "any list fits a bare list", "one function that ranks the definitions of a
# word"): a value whose type nothing settles is an any, and a program means the same whether its types are known.
# Run it with bash test/wap/test_dynamic.sh
source "$(dirname "$0")/compile.sh"

DESCRIBE='describe(x: int) := "a number"
describe(x: text) := "a text"
describe(x: T?) := "maybe something"'

check "items of two types make a list of anything" "$(ran mixed 'xs = ["ab", 1]
print text(#xs)')" "2"
check "a known and an unknown value choose the same definition" "$(ran same_choice "$DESCRIBE"'
xs = ["ab", 3]
print describe(3)
print describe(xs#2)')" "a number
a number"
check "an unknown value chooses by what it is" "$(ran chosen "$DESCRIBE"'
xs = ["ab", 3]
print describe(xs#1)')" "a text"
check "an operator of two unknown values" "$(ran two_unknown 'xs = ["ab", 3, 4]
print text(xs#2 + xs#3)')" "7"
check "a typed list fits a field typed list" "$(ran bare_field 'class Shelf { items: list }
shelf = Shelf([1, 2])
print text(#shelf.items)')" "2"
check "a list of anything fits a parameter typed list" "$(ran bare_parameter 'size(xs: list) := #xs
print text(size(["ab", 1]))')" "2"
check "a field read on an unknown value" "$(ran unknown_field 'class Pen { name: text }
xs = [Pen("blue"), 3]
print xs#1.name')" "blue"
check "an unknown value given to a typed name is checked there" "$(ran checked_in 'xs = ["ab", 3]
n: int = xs#2
print text(n + 1)')" "4"
check "an unknown value beside a known one chooses by both" "$(ran beside_known 'e: any = 2.5
print text(e == 2.5)
print text(e == 3.5)')" "yes
no"
