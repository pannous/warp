# other languages' declaration forms (src/warp_parser/definitions/foreign.rs), one program a line
class Circle{r:int; static k = 3}; Circle.k
static k = 3; k
private x = 4; x
public static fun f() := 3; f()
public static int x = 5; x
export fun g() := 2; g()
global x = 1; x
class P{x:int}; new P(1)
constant x = 3; x
class P { attr_accessor :x, :y }; P(1, 2).y
enum class Color {red, green}; green
data class P {x:int}; P(1).x
open class P {x:int}; P(2).x
abstract class Shape {n:int}; Shape(3).n
mixin M { f() := 1 }
public = 3; public
static(3)
# Later: Kotlin primary constructors (card self-primary-constructor), operator heads (card self-operator-heads)
