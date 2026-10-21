[1, 2, 3].sum { |x| x * 2 }
[1, 2, 3].count { |x| x > 1 }
f = |x| x*2; f(3)
f = |a, b| a*b; f(2,3)
f = |a b| a+b; f(2,3)
fold([1,2,3], 0, { |acc, x| acc + x })
total = 0⏎def each_twice⏎  yield 1⏎  yield 2⏎end⏎each_twice { |x| total += x }⏎total
def twice(n)⏎  yield n⏎  yield n * 2⏎end⏎sum = 0⏎twice(3) { |x| sum += x }⏎sum
def apply(n)⏎  yield n⏎end⏎apply(3) do |x| x * 2 end
def pair⏎  yield 3, 4⏎end⏎pair { |a, b| a * b }
def run⏎  yield⏎end⏎run { 5 }
map([1,2,3], |x| x*x)
[1,2,3,4].filter(|x| x % 2 == 0)
map([1,2,3], { |x| x*x })
[1,2,3].map { |x| x+1 }
