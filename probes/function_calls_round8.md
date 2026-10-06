# round 8 of ported call forms: code ||| expected
local function sq(x) return x * x end; sq(5) ||| 25
function sq(x) return x * x end; sq(6) ||| 36
sq = function(x) return x * x end; sq(3) ||| 9
fun sq(x: Int): Int = x * x; sq(4) ||| 16
fun sq(x: Int) = x * x; sq(5) ||| 25
val sq = { x: Int -> x * x }; sq(3) ||| 9
listOf(1, 2, 3).map { it * 2 } ||| [2 4 6]
def sq(x), do: x * x; sq(3) ||| 9
sq = fn x -> x * x end; sq.(7) ||| 49
Enum.map([1, 2, 3], fn x -> x * 2 end) ||| [2 4 6]
Enum.map([1, 2, 3], &(&1 * 2)) ||| [2 4 6]
sq x = x * x; sq 4 ||| 16
map (\x -> x * 2) [1, 2, 3] ||| [2 4 6]
(\x -> x + 1) 4 ||| 5
function sq($x) { return $x * $x; } sq(3); ||| 9
$sq = fn($x) => $x * $x; $sq(4) ||| 16
array_map(fn($x) => $x * 2, [1, 2, 3]) ||| [2 4 6]
# not ported (s-expressions, Perl @_, Scala _ pending P160): sub sq { my ($x) = @_; return $x * $x; } sq(5) ||| 25
sq(x) = x^2; sq.([1, 2, 3]) ||| [1 4 9]
function sq(x) x^2 end; sq(3) ||| 9
# not ported (s-expressions, Perl @_, Scala _ pending P160): (defn sq [x] (* x x)) (sq 4) ||| 16
# not ported (s-expressions, Perl @_, Scala _ pending P160): (map (fn [x] (* x 2)) [1 2 3]) ||| [2 4 6]
# not ported (s-expressions, Perl @_, Scala _ pending P160): (define (sq x) (* x x)) (sq 5) ||| 25
# not ported (s-expressions, Perl @_, Scala _ pending P160): (lambda (x) (* x x)) ||| 
static int Sq(int x) => x * x; Sq(3) ||| 9
Func<int, int> sq = x => x * x; sq(4) ||| 16
auto sq = [](int x) { return x * x; }; sq(5) ||| 25
std::function<int(int)> sq = [](int x) { return x * x; }; sq(6) ||| 36
def sq(x: Int): Int = x * x; sq(3) ||| 9
val sq = (x: Int) => x * x; sq(4) ||| 16
# not ported (s-expressions, Perl @_, Scala _ pending P160): List(1, 2, 3).map(_ * 2) ||| [2 4 6]
[1, 2, 3].map(&:to_s) ||| ["1" "2" "3"]
[1, 2, 3].sum { |x| x * 2 } ||| 12
