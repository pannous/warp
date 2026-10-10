import WarpTypes.Typing

/-! Small-step semantics of W0: left to right, call by value. An `error` in any evaluation position propagates,
except under `try`; a stored error (`fail`) is a value until an operation that needs another value meets it. Main-level names are cells of the store; class instances live in its heap, which only grows. -/

namespace Warp
open Ty Expr

inductive Cell where
  | unset
  | val (v : Expr)
  /-- the body of a charged name `z := e`, run at each read -/
  | charged (body : Expr)

/-- a class instance: its class chain and the fields set so far -/
structure Obj where
  path : List String
  fields : String → Option Expr

structure Store where
  vars : String → Option Cell
  heap : List Obj := []
  /-- the active block handlers, innermost first -/
  handlers : List (String × Expr) := []
  /-- the shared lists: each one's element type and its items (a `nil`/`cons` value); only grows -/
  lists : List (Ty × Expr) := []

instance : CoeFun Store (fun _ => String → Option Cell) := ⟨Store.vars⟩

def Store.set (μ : Store) (x : String) (c : Cell) : Store :=
  { μ with vars := fun z => if z = x then some c else μ.vars z }

/-- a new instance with no field set, at address `μ.heap.length` -/
def Store.alloc (μ : Store) (p : List String) : Store := { μ with heap := μ.heap ++ [⟨p, fun _ => none⟩] }

/-- the object at address a, if it is of class chain p -/
def Store.obj (μ : Store) (a : Nat) (p : List String) : Option Obj := μ.heap[a]?.filter (·.path == p)

/-- the items of the shared list at address a, if it was made with element type t -/
def Store.listAt (μ : Store) (a : Nat) (t : Ty) : Option Expr := (μ.lists[a]?.filter (·.1 == t)).map (·.2)

/-- a new shared list, at address `μ.lists.length` -/
def Store.allocList (μ : Store) (t : Ty) (items : Expr) : Store := { μ with lists := μ.lists ++ [(t, items)] }

def Store.writeList (μ : Store) (a : Nat) (items : Expr) : Store :=
  { μ with lists := μ.lists.modify a fun c => (c.1, items) }

/-- what a list operation reads: a shared list's items (`nil` for an address no run makes), any other value itself -/
def Store.items (μ : Store) : Expr → Expr
  | .lref a t => (μ.listAt a t).getD .nil
  | v => v

def Store.withHandlers (μ : Store) (hs : List (String × Expr)) : Store := { μ with handlers := hs }

def Store.push (μ : Store) (ev : String) (h : Expr) : Store := μ.withHandlers ((ev, h) :: μ.handlers)

/-- only the k outermost handlers active -/
def Store.outer (μ : Store) (k : Nat) : Store := μ.withHandlers (μ.handlers.drop (μ.handlers.length - k))

/-- the innermost handler of ev and the number of handlers outside it -/
def lookupHandler : List (String × Expr) → String → Option (Expr × Nat)
  | [], _ => none
  | (e, h) :: rest, ev => if e = ev then some (h, rest.length) else lookupHandler rest ev

theorem lookupHandler_mem : ∀ {hs : List (String × Expr)} {ev h k}, lookupHandler hs ev = some (h, k) → (ev, h) ∈ hs
  | (e, g) :: rest, ev, h, k, hl => by
    simp only [lookupHandler] at hl
    split at hl
    · cases hl; subst_vars; exact List.mem_cons_self
    · exact List.mem_cons_of_mem _ (lookupHandler_mem hl)

def Store.write (μ : Store) (a : Nat) (f : String) (v : Expr) : Store :=
  { μ with heap := μ.heap.modify a fun o => { o with fields := fun g => if g = f then some v else o.fields g } }

/-- `o.f` of a value o -/
def readField (μ : Store) : Expr → String → Expr
  | .ref a p, f =>
    match μ.obj a p with
    | some o => (o.fields f).getD (.error "unset field")
    | none => .error "dangling reference"
  | _, _ => .error "not an object"

/-- `o.f = v` of values o and v -/
def writeField (μ : Store) : Expr → String → Expr → Expr × Store
  | .ref a p, f, v => if (μ.obj a p).isSome then (v, μ.write a f v) else (.error "dangling reference", μ)
  | _, _, _ => (.error "not an object", μ)

/-- `v is c`: c is the class of v or one of its ancestors -/
def isInstance : Expr → String → Bool
  | .ref _ p, c => p.contains c
  | _, _ => false

/-- true/false act as 1/0 -/
def asInt : Expr → Option Int
  | .bool b => some (if b then 1 else 0)
  | .int n => some n
  | _ => none

/-- an exact number's value: an int's, a rational's -/
def asExact : Expr → Option Rat
  | .num q => some q
  | e => (asInt e).map fun n => (n : Rat)

def ratToFloat (q : Rat) : Float := Float.ofInt q.num / Float.ofNat q.den

/-- a number's value as a float -/
def asFloat : Expr → Float
  | .flt f => Float.ofBits f
  | e => ratToFloat ((asExact e).getD 0)

/-- a float's whole part, toward zero (`3.7 as int` is 3) -/
def floatWhole (f : Float) : Int :=
  let whole := (if f < 0 then -f else f).floor
  let magnitude : Int := if whole < 18446744073709551616.0 then whole.toUInt64.toNat else 0
  if f < 0 then -magnitude else magnitude

/-- a number's whole part, toward zero -/
def asNumber : Expr → Int
  | .num q => if q < 0 then -((-q).floor) else q.floor
  | .flt f => floatWhole (Float.ofBits f)
  | e => (asInt e).getD 0

/-- an exact number as a value: a whole one is an int, as warp normalizes `0.5 * 2` -/
def exactValue (q : Rat) : Expr := if q.den = 1 then .int q.num else .num q

def floatValue (f : Float) : Expr := .flt f.toBits

/-- the multiplicity of the prime p in n (n > 0) -/
def multiplicity (p : Nat) (n : Nat) (fuel : Nat := 64) : Nat :=
  match fuel with
  | 0 => 0
  | fuel + 1 => if n > 0 && n % p == 0 then 1 + multiplicity p (n / p) fuel else 0

/-- a rational as warp prints it: a terminating decimal (`3.5`, `0.125`), else as a fraction (`1/3`) -/
def ratDisplay (q : Rat) : String :=
  let twos := multiplicity 2 q.den
  let fives := multiplicity 5 q.den
  if q.den != 2 ^ twos * 5 ^ fives then s!"{q.num}/{q.den}" else
  let digits := max twos fives
  let scaled := q.num.natAbs * (10 ^ digits / q.den)
  let whole := toString (scaled / 10 ^ digits)
  let fraction := toString (scaled % 10 ^ digits)
  let padded := String.ofList (List.replicate (digits - fraction.length) '0') ++ fraction
  (if q.num < 0 then "-" else "") ++ whole ++ (if digits = 0 then "" else "." ++ padded)

/-- warp's float_text constants (src/wasm_emitter/float_text.rs): significant digits, positional exponents -/
def significantDigits : Nat := 15
def smallestPositional : Int := -5
def largestPositional : Int := 15

/-- x = mantissa · 10^exponent, the mantissa in [1, 10), scaled by ×10 and ÷10 steps as float_text does -/
def decimalScale : (fuel : Nat) → Float → Int → Float × Int
  | 0, x, exponent => (x, exponent)
  | fuel + 1, x, exponent =>
    if x >= 10 then decimalScale fuel (x / 10) (exponent + 1)
    else if x < 1 then decimalScale fuel (x * 10) (exponent - 1)
    else (x, exponent)

/-- a positive float rounded to the nearest whole, ties to even (wasm's f64.nearest) -/
def roundHalfEven (y : Float) : Float :=
  let r := y.round
  if (r - y).abs == 0.5 && (r / 2).floor * 2 != r then r - 1 else r

/-- n without its trailing zeros, keeping at least one digit, and how many digits are left -/
def dropTrailingZeros : (fuel : Nat) → Nat → Nat → Nat × Nat
  | 0, n, count => (n, count)
  | fuel + 1, n, count => if count > 1 && n % 10 == 0 then dropTrailingZeros fuel (n / 10) (count - 1) else (n, count)

def zeroText (count : Int) : String := String.ofList (List.replicate count.toNat '0')

/-- the digits of a positive float laid out at its decimal exponent: positional from 1e-5 up to 1e15, else `1.5e20` -/
def placeDigits (digits : String) (exponent : Int) : String :=
  let count : Int := digits.length
  let before (k : Int) := String.ofList (digits.toList.take k.toNat)
  let after (k : Int) := String.ofList (digits.toList.drop k.toNat)
  if exponent < smallestPositional || exponent >= largestPositional then
    before 1 ++ (if count > 1 then "." ++ after 1 else "") ++ "e" ++ toString exponent
  else if exponent >= count - 1 then digits ++ zeroText (exponent - count + 1)
  else if exponent >= 0 then before (exponent + 1) ++ "." ++ after (exponent + 1)
  else "0." ++ zeroText (-1 - exponent) ++ digits

/-- a float as warp's float_text writes it: at most 15 significant digits (`0.5`, `1.5e-7`), as `%.15g` -/
def floatText (f : Float) : String :=
  if f.isNaN then "NaN" else if f.isInf then (if f > 0 then "∞" else "-∞") else if f == 0 then "0" else
  let (mantissa, exponent) := decimalScale 700 f.abs 0
  let scaled := (roundHalfEven (mantissa * 1e14)).toUInt64.toNat
  let (whole, exponent) := if scaled >= 10 ^ significantDigits then (scaled / 10, exponent + 1) else (scaled, exponent)
  let (digits, _) := dropTrailingZeros significantDigits whole significantDigits
  (if f < 0 then "-" else "") ++ placeDigits (toString digits) exponent

/-- a value as `+` joins it to a text -/
def render : Expr → String
  | .text s => s
  | .bool b => if b then "true" else "false"
  | .int n => toString n
  | .num q => ratDisplay q
  | .flt f => floatText (Float.ofBits f)
  | _ => ""

def isText : Expr → Bool
  | .text _ => true
  | _ => false

/-- a text's first character and the rest: `for c in "ab"` walks its one-character texts -/
def peel (s : String) : Option (String × String) :=
  match s.toList with
  | [] => none
  | c :: cs => some (c.toString, String.ofList cs)

/-- a walk over a text gives one-character texts: codepoints -/
theorem peel_codepoint {s c rest : String} (h : peel s = some (c, rest)) : c.length = 1 := by
  unfold peel at h
  split at h
  · cases h
  · cases h; simp [Char.toString]

/-- one step of a walk over a text: done at the end, else the body for the first character, then the rest -/
def walkText (y : String) (s : String) (b last : Expr) : Expr :=
  match peel s with
  | none => last
  | some (c, rest) => .forIn y (.text rest) b (b.subst y (.text c))

def isNumber : Expr → Bool
  | .bool _ | .int _ | .num _ | .flt _ => true
  | _ => false

/-- a zero divisor: `x / 0`, `x % 0.0` -/
def isZero : Expr → Bool
  | .flt f => Float.ofBits f == 0
  | v => asExact v == some 0

/-- `"ab" * 3`, `3 * "ab"`: the text n times (none for n ≤ 0), P1 -/
def repeatValues (s : String) (n : Expr) : Expr :=
  match asInt n with
  | some k => .text (String.join (List.replicate k.toNat s))
  | none => .error "a text repeats a whole number of times"

def isQuantityValue : Expr → Bool
  | .qty _ _ => true
  | _ => false

/-- a quantity's dimensions, a number's none (`[]`) -/
def valueDims : Expr → Option Dims
  | .qty _ d => some d
  | v => if isNumber v then some [] else none

def amount : Expr → Int
  | .qty n _ => n
  | v => asNumber v

/-- the value of an amount in dimensions d: a number when they cancel -/
def quantityValue (n : Int) (d : Dims) : Expr := if d = [] then .int n else .qty n d

def DIMENSION_ERROR : String := "DimensionError"

/-- `-`, `*` and `/` with a quantity side (ArithOp.quantityTy) -/
def quantityValues (op : ArithOp) (a b : Expr) : Expr :=
  if op = .sub then
    match a, b with
    | .qty x d, .qty y e => if d = e then .qty (x - y) d else .error DIMENSION_ERROR
    | _, _ => .error DIMENSION_ERROR
  else
  match valueDims a, valueDims b with
  | some d, some e =>
    match op.dims d e with
    | some dims => if op = .div && amount b == 0 then .error "divide by zero" else quantityValue (op.apply (amount a) (amount b)) dims
    | none => .error DIMENSION_ERROR
  | _, _ => .error DIMENSION_ERROR

/-- an operation on exact numbers, a float when the result is no rational (`2^0.5`) -/
def exactNumber (op : ArithOp) (x y : Rat) : Expr :=
  match op.applyExact x y with
  | some q => exactValue q
  | none => floatValue (op.applyFloat (ratToFloat x) (ratToFloat y))

/-- `-`, `*`, `%`, `/` and `^` without quantities: numbers, or a text repeated -/
def plainArithValues (op : ArithOp) (a b : Expr) : Expr :=
  match op, a, b with
  | .mul, .text s, n | .mul, n, .text s => repeatValues s n
  | _, _, _ => numberValues op a b
where numberValues (op : ArithOp) (a b : Expr) : Expr :=
  if !(isNumber a && isNumber b) then .error "not a number" else
  if (op == .mod || op == .div) && isZero b then .error "divide by zero" else
  match asInt a, asInt b with
  | some x, some y => if (op == .div && x % y != 0) || (op == .pow && y < 0) then exactNumber op x y else .int (op.apply x y)
  | _, _ =>
  match asExact a, asExact b with
  | some x, some y => exactNumber op x y
  | _, _ => floatValue (op.applyFloat (asFloat a) (asFloat b))

/-- `-`, `*`, `%`, `/` and `^` on values -/
def arithValues (op : ArithOp) (a b : Expr) : Expr :=
  if (isQuantityValue a || isQuantityValue b) = true then quantityValues op a b else plainArithValues op a b

def isList : Expr → Bool
  | .nil | .cons _ _ => true
  | _ => false

def concat : Expr → Expr → Expr
  | .cons h t, b => .cons h (concat t b)
  | _, b => b

/-- `+`: two lists concatenate (`xs + ys`), numbers add, a text side makes a text -/
def addValues (a b : Expr) : Expr :=
  match a, b with
  | .qty x d, .qty y e => if d = e then .qty (x + y) d else .error DIMENSION_ERROR
  | _, _ =>
  if isList a && isList b then concat a b else
  if !((isNumber a || isText a) && (isNumber b || isText b)) then .error "not addable" else
  if isText a || isText b then .text (render a ++ render b) else
  match asInt a, asInt b with
  | some x, some y => .int (x + y)
  | _, _ =>
  match asExact a, asExact b with
  | some x, some y => exactValue (x + y)
  | _, _ => floatValue (asFloat a + asFloat b)

/-- numbers by value, texts in codepoint order (`"a" < "b"`) -/
def ltValues (a b : Expr) : Expr :=
  match a, b with
  | .qty x d, .qty y e => if d = e then .bool (decide (x < y)) else .error DIMENSION_ERROR
  | _, _ =>
  if isNumber a && isNumber b then
    match asExact a, asExact b with
    | some x, some y => .bool (decide (x < y))
    | _, _ => .bool (asFloat a < asFloat b)
  else
  match a, b with
  | .text x, .text y => .bool (decide (x < y))
  | _, _ => .error "not comparable"

def EQ_FUEL : Nat := 64

/-- `==` is loose (P208): instances of one class whose fields are equal are equal, lists item by item; `same` is
identity. Fuel bounds the walk through cyclic instances -/
def looseEq (P : Program) (μ : Store) : Nat → Expr → Expr → Bool
  | fuel + 1, .ref a p, .ref b q =>
    a == b || (p == q && match μ.obj a p, μ.obj b q with
      | some o, some r => (P.fieldNames p).all fun f => match o.fields f, r.fields f with
        | some x, some y => looseEq P μ fuel x y
        | none, none => true
        | _, _ => false
      | _, _ => false)
  | fuel + 1, .lref a t, b => looseEq P μ fuel (μ.items (.lref a t)) b
  | fuel + 1, a, .lref b u => looseEq P μ fuel a (μ.items (.lref b u))
  | fuel + 1, .cons h t, .cons h' t' => looseEq P μ fuel h h' && looseEq P μ fuel t t'
  | _, a, b => decide (a = b)

/-- `same` compares by identity (an instance is its address), `==` loosely -/
def eqValues (P : Program) (μ : Store) (same : Bool) (a b : Expr) : Bool :=
  if same then decide (a = b) else looseEq P μ EQ_FUEL a b

/-- ø: warp's ø is the empty list (`[] ?? 3` is 3) -/
def isEmpty : Expr → Bool
  | .unit | .nil => true
  | _ => false

/-- a stored error -/
def isFail : Expr → Bool
  | .fail _ => true
  | _ => false

theorem isFail_eq : ∀ {e : Expr}, isFail e = true → ∃ m, e = .fail m := by
  intro e h; cases e <;> simp_all [isFail]

/-- false, 0, "", ø, [] and a stored error are falsy -/
def truthy : Expr → Bool
  | .bool b => b
  | .int n => n != 0
  | .num q => q != 0
  | .flt f => Float.ofBits f != 0
  | .text s => !s.isEmpty
  | .unit | .nil | .fail _ => false
  | _ => true

/-- the i-th element, 1-based; of a text its i-th codepoint -/
def nth : Expr → Int → Option Expr
  | .cons h t, i => if i = 1 then some h else nth t (i - 1)
  | .text s, i => if 1 ≤ i then s.toList[(i - 1).toNat]?.map fun c => .text (String.singleton c) else none
  | _, _ => none

def isClosure : Expr → Bool
  | .clo _ _ => true
  | _ => false

def isShared : Expr → Bool
  | .lref _ _ => true
  | _ => false

def appendValues (a b : Expr) : Expr := if isList a && isList b then concat a b else .error "not a list"

/-- the type of a value, as `HasType` gives it -/
def valueType : Expr → Option Ty
  | .bool _ => some .bool
  | .int _ => some .int
  | .num _ => some .exact
  | .flt _ => some .number
  | .qty _ d => some (.quantity d)
  | .text s => some (textTy s)
  | .unit => some .unit
  | .nil => some (.list .never)
  | .ref _ p => some (.cls p)
  | .lref _ t => some (.list t)
  | .fail _ => some .any
  | .cons h t =>
    match valueType h, valueType t with
    | some a, some l =>
      match element l with
      | some e => some (.list (join a e))
      | none => none
    | _, _ => none
  | _ => none

/-- the run-time type test of a cast: an int fits a fixed width when it is in its range, a list fits a list type when
each item fits its element type, any other value when its type is below t -/
def fits : Expr → Ty → Bool
  | .int n, .ranged lo hi => decide (lo ≤ n ∧ n ≤ hi)
  | .cons h t, .list e => fits h e && fits t (.list e)
  | v, t =>
    match valueType v with
    | some tv => sub tv t
    | none => false

/-- a whole float prints as its int (`x: float = 2` is 2); W0 does not print other floats as warp does -/
def floatDisplay (f : Float) : String :=
  if f == f.floor && f.abs < 9007199254740992.0 then toString (floatWhole f) else "?"

/-- a value as warp prints it, shared lists read in store μ down to `depth` levels; `?` where the model does not keep
what warp prints (numbers, instances) -/
def display (μ : Store) : Nat → Expr → String
  | _, .bool b => if b then "yes" else "no"
  | _, .int n => toString n
  | _, .num q => ratDisplay q
  | _, .flt f => floatDisplay (Float.ofBits f)
  | _, .text s => s!"\"{s}\""
  | _, .fail _ => "error"
  | depth + 1, .lref a t => display μ depth (μ.items (.lref a t))
  | depth, .cons h t => "[" ++ " ".intercalate (showItems depth (.cons h t)) ++ "]"
  | _, _ => "?"
where showItems (depth : Nat) : Expr → List String
  | .cons h t => display μ depth h :: showItems depth t
  | _ => []

/-- how deep `display` follows shared lists: a list that holds itself prints `?` there -/
def DISPLAY_DEPTH : Nat := 8

/-- `v as text`: a text as it is, the empty list ø, anything else as warp prints it -/
def textForm (μ : Store) (v : Expr) : String :=
  match μ.items v with
  | .text s | .fail s => s
  | .nil => "ø"
  | v => display μ DISPLAY_DEPTH v

/-- `v as t` of a value: a scalar converts to text, int, number or bool (a number to int keeps its whole part, a text
is parsed); to any other type the value is checked as a cast checks it -/
def convertValue (μ : Store) (v : Expr) : Ty → Expr
  | .text => .text (textForm μ v)
  | .bool => .bool (truthy (μ.items v))
  | .int => if isNumber v then .int (asNumber v) else parsed .int v
  | .number => if isNumber v then floatValue (asFloat v) else parsed (fun n => floatValue (Float.ofInt n)) v
  | t => if fits v t then v else .error "cannot cast"
where parsed (make : Int → Expr) : Expr → Expr
  | .text s => (s.toInt?.map make).getD (.error "invalid number")
  | _ => .error "cannot cast"

/-- `xs.add(v)` of values: v joins the shared list if it fits the list's element type -/
def pushValues (μ : Store) : Expr → Expr → Expr × Store
  | .lref a t, v =>
    match μ.listAt a t with
    | some items => if fits v t then (.lref a t, μ.writeList a (concat items (.cons v .nil))) else (.error "type mismatch", μ)
    | none => (.error "dangling list", μ)
  | _, _ => (.error "not a list", μ)

/-- the list value xs with its n-th item (from 1) replaced by v, if it has one -/
def replaceAt : Expr → Int → Expr → Option Expr
  | .cons h t, n, v => if n = 1 then some (.cons v t) else (replaceAt t (n - 1) v).map (.cons h)
  | _, _, _ => none

/-- `xs#i` of values: the i-th item of a list or a text, from 1; `m[k]` of an instance its field k (a map's computed
key) -/
def indexValues (μ : Store) (l i : Expr) : Expr :=
  match l, i with
  | .ref a p, .text k => readField μ (.ref a p) k
  | _, _ => (nth (μ.items l) ((asInt i).getD 0)).getD (.error "index out of range")

/-- `xs#i = v` of values: v replaces an item of the shared list if it fits the list's element type; `m[k] = v` the
field k of an instance if its class has one and v fits its type -/
def setAtValues (P : Program) (μ : Store) : Expr → Expr → Expr → Expr × Store
  | .lref a t, i, v =>
    match μ.listAt a t with
    | some items =>
      if fits v t then
        match replaceAt items ((asInt i).getD 0) v with
        | some items' => (v, μ.writeList a items')
        | none => (.error "index out of range", μ)
      else (.error "type mismatch", μ)
    | none => (.error "dangling list", μ)
  | .ref a p, .text k, v =>
    match P.fieldTy p k with
    | some t => if (μ.obj a p).isSome && fits v t then (v, μ.write a k v) else (.error "type mismatch", μ)
    | none => (.error "no field", μ)
  | _, _, _ => (.error "not a list", μ)

/-- the k ints from m: `[m, m+1, …]` -/
def intList (m : Int) : Nat → Expr
  | 0 => .nil
  | k + 1 => .cons (.int m) (intList (m + 1) k)

/-- `a..b` of two values: the ints from a up to b, an error unless both are ints -/
def rangeValues (a b : Expr) : Expr :=
  match asInt a, asInt b with
  | some m, some n => intList m (n - m).toNat
  | _, _ => .error "not an int"

/-- an evaluation position: the hole is evaluated next once the expressions left of it are values -/
inductive Frame where
  | consL (t : Expr) | consR (h : Expr)
  | addL (b : Expr) | addR (a : Expr)
  | arithL (op : ArithOp) (b : Expr) | arithR (op : ArithOp) (a : Expr)
  | ltL (b : Expr) | ltR (a : Expr)
  | eqL (s : Bool) (b : Expr) | eqR (s : Bool) (a : Expr)
  | ite (a b : Expr)
  | seq (b : Expr)
  | indexL (i : Expr) | indexR (l : Expr)
  | rangeL (b : Expr) | rangeR (a : Expr)
  | appendL (b : Expr) | appendR (a : Expr)
  | assign (x : String) | init (x : String)
  | letIn (y : String) (t : Ty) (b : Expr)
  | call (f : String)
  | cast (ts : List Ty)
  | conv (t : Ty)
  | broadcast (f : String)
  | get (f : String)
  | setL (f : String) (v : Expr) | setR (o : Expr) (f : String)
  | isA (c : String)
  | failed
  | orElse (b : Expr)
  | emit (ev : String)
  | abort (ev : String) (k : Option Nat)
  | forIn (y : String) (b last : Expr)
  | loopLast (c b : Expr)
  | forInLast (y : String) (l b : Expr)
  | appL (a : Expr) | appR (f : Expr)
  | share (t : Ty)
  | pushL (v : Expr) | pushR (l : Expr)
  | setAtL (i v : Expr) | setAtI (l v : Expr) | setAtR (l i : Expr)

namespace Frame

def plug : Frame → Expr → Expr
  | consL t, e => .cons e t
  | consR h, e => .cons h e
  | addL b, e => .add e b
  | addR a, e => .add a e
  | arithL op b, e => .arith op e b
  | arithR op a, e => .arith op a e
  | ltL b, e => .lt e b
  | ltR a, e => .lt a e
  | eqL s b, e => .eq s e b
  | eqR s a, e => .eq s a e
  | ite a b, e => .ite e a b
  | seq b, e => .seq e b
  | indexL i, e => .index e i
  | indexR l, e => .index l e
  | rangeL b, e => .range e b
  | rangeR a, e => .range a e
  | appendL b, e => .append e b
  | appendR a, e => .append a e
  | assign x, e => .assign x e
  | init x, e => .init x e
  | letIn y t b, e => .letIn y t e b
  | call f, e => .call f e
  | cast ts, e => .cast e ts
  | conv t, e => .conv e t
  | broadcast f, e => .broadcast f e
  | get f, e => .get e f
  | setL f v, e => .set e f v
  | setR o f, e => .set o f e
  | isA c, e => .isA e c
  | failed, e => .failed e
  | orElse b, e => .orElse e b
  | emit ev, e => .emit ev e
  | abort ev k, e => .abort ev k e
  | forIn y b d, e => .forIn y e b d
  | loopLast c b, e => .loop c b e
  | forInLast y l b, e => .forIn y l b e
  | appL a, e => .app e a
  | appR f, e => .app f e
  | share t, e => .share e t
  | pushL v, e => .push e v
  | pushR l, e => .push l e
  | setAtL i v, e => .setAt e i v
  | setAtI l v, e => .setAt l e v
  | setAtR l i, e => .setAt l i e

/-- a right position needs the left operand evaluated -/
def ready : Frame → Bool
  | consR h | addR h | arithR _ h | ltR h | eqR _ h | indexR h | rangeR h | appendR h | appR h | setR h _ | pushR h | setAtI h _
  | forInLast _ h _ => h.isValue
  | setAtR l i => l.isValue && i.isValue
  | _ => true

end Frame

inductive Step (P : Program) : Expr × Store → Expr × Store → Prop where
  | frame {F : Frame} {e e' μ μ'} : F.ready = true → Step P (e, μ) (e', μ') → Step P (F.plug e, μ) (F.plug e', μ')
  | raise {F : Frame} {m μ} : F.ready = true → Step P (F.plug (.error m), μ) (.error m, μ)
  | tryStep {e e' h μ μ'} : Step P (e, μ) (e', μ') → Step P (.tryCatch e h, μ) (.tryCatch e' h, μ')
  | tryError {m h μ} : Step P (.tryCatch (.error m) h, μ) (h, μ)
  | tryValue {v h μ} : v.isValue = true → isFail v = false → Step P (.tryCatch v h, μ) (v, μ)
  | tryFail {m h μ} : Step P (.tryCatch (.fail m) h, μ) (h, μ)
  | readValue {x v μ} : μ x = some (.val v) → Step P (.glob x, μ) (v, μ)
  | readUnset {x μ} : μ x = some .unset → Step P (.glob x, μ) (.error "unset", μ)
  | readCharged {x b μ} : μ x = some (.charged b) → Step P (.glob x, μ) (b, μ)
  | add {a b μ} : a.isValue = true → b.isValue = true → Step P (.add a b, μ) (addValues (μ.items a) (μ.items b), μ)
  | arith {op a b μ} : a.isValue = true → b.isValue = true → Step P (.arith op a b, μ) (arithValues op a b, μ)
  | lt {a b μ} : a.isValue = true → b.isValue = true → Step P (.lt a b, μ) (ltValues a b, μ)
  | eq {s a b μ} : a.isValue = true → b.isValue = true → Step P (.eq s a b, μ) (.bool (eqValues P μ s a b), μ)
  | ite {v a b μ} : v.isValue = true → Step P (.ite v a b, μ) (if truthy (μ.items v) then a else b, μ)
  | loop {c b v μ} : v.isValue = true → Step P (.loop c b v, μ) (.ite c (.loop c b b) v, μ)
  | seq {v b μ} : v.isValue = true → Step P (.seq v b, μ) (b, μ)
  | lam {y b μ} : Step P (.lam y b, μ) (.clo y b, μ)
  | app {y b v μ} : v.isValue = true → Step P (.app (.clo y b) v, μ) (b.subst y v, μ)
  | appOther {f v μ} : f.isValue = true → v.isValue = true → isClosure f = false →
      Step P (.app f v, μ) (.error "not a function", μ)
  | range {a b μ} : a.isValue = true → b.isValue = true → Step P (.range a b, μ) (rangeValues a b, μ)
  | index {l i μ} : l.isValue = true → i.isValue = true →
      Step P (.index l i, μ) (indexValues μ l i, μ)
  | append {a b μ} : a.isValue = true → b.isValue = true →
      Step P (.append a b, μ) (appendValues (μ.items a) (μ.items b), μ)
  | assign {x v μ} : v.isValue = true → Step P (.assign x v, μ) (v, μ.set x (.val v))
  | init {x v μ} : v.isValue = true → Step P (.init x v, μ) (v, μ.set x (.val v))
  | letIn {y t v b μ} : v.isValue = true → Step P (.letIn y t v b, μ) (b.subst y v, μ)
  | call {f v fn μ} : v.isValue = true → P.funs f = some fn →
      Step P (.call f v, μ) (.letIn fn.param fn.paramTy v fn.body, μ)
  | broadcastNil {f μ} : Step P (.broadcast f .nil, μ) (.nil, μ)
  | broadcastCons {f h t μ} : h.isValue = true → t.isValue = true →
      Step P (.broadcast f (.cons h t), μ) (.cons (.call f h) (.broadcast f t), μ)
  | broadcastRef {f a t μ} : Step P (.broadcast f (.lref a t), μ) (.broadcast f (μ.items (.lref a t)), μ)
  | shareNew {v t μ} : v.isValue = true → isList v = true → fits v (.list t) = true →
      Step P (.share v t, μ) (.lref μ.lists.length t, μ.allocList t v)
  | shareBad {v t μ} : v.isValue = true → (isList v && fits v (.list t)) = false →
      Step P (.share v t, μ) (.error "type mismatch", μ)
  | push {l v μ} : l.isValue = true → v.isValue = true → Step P (.push l v, μ) (pushValues μ l v)
  | setAt {l i v μ} : l.isValue = true → i.isValue = true → v.isValue = true →
      Step P (.setAt l i v, μ) (setAtValues P μ l i v)
  | cast {v ts μ} : v.isValue = true → Step P (.cast v ts, μ) (if ts.any (fits v) then v else .error "type mismatch", μ)
  | conv {v t μ} : v.isValue = true → Step P (.conv v t, μ) (convertValue μ v t, μ)
  | new {p μ} : Step P (.new p, μ) (.ref μ.heap.length p, μ.alloc p)
  | get {o f μ} : o.isValue = true → Step P (.get o f, μ) (readField μ o f, μ)
  | set {o f v μ} : o.isValue = true → v.isValue = true → Step P (.set o f v, μ) (writeField μ o f v)
  | isA {v c μ} : v.isValue = true → Step P (.isA v c, μ) (.bool (isInstance v c), μ)
  | failed {v μ} : v.isValue = true → Step P (.failed v, μ) (.bool (isFail v), μ)
  | orElse {v b μ} : v.isValue = true → Step P (.orElse v b, μ) (if isEmpty (μ.items v) then b else v, μ)
  /-- the body runs with the handler pushed; the handlers return to what they were -/
  | handleStep {ev h b b' μ μ'} : Step P (b, μ.push ev h) (b', μ') →
      Step P (.handle ev h b, μ) (.handle ev h b', μ'.withHandlers μ.handlers)
  | handleValue {ev h v μ} : v.isValue = true → Step P (.handle ev h v, μ) (v, μ)
  | handleError {ev h m μ} : Step P (.handle ev h (.error m), μ) (.error m, μ)
  | emitBlock {ev v h k μ} : v.isValue = true → lookupHandler μ.handlers ev = some (h, k) →
      Step P (.emit ev v, μ) (.scope k (h.subst eventLocal v), μ)
  | emitProgram {ev v h μ} : v.isValue = true → lookupHandler μ.handlers ev = none → P.handlers ev = some h →
      Step P (.emit ev v, μ) (.scope 0 (h.subst eventLocal v), μ)
  | emitNone {ev v μ} : v.isValue = true → lookupHandler μ.handlers ev = none → P.handlers ev = none →
      Step P (.emit ev v, μ) (.unit, μ)
  | scopeStep {k e e' μ μ'} : Step P (e, μ.outer k) (e', μ') →
      Step P (.scope k e, μ) (.scope k e', μ'.withHandlers μ.handlers)
  | scopeValue {k v μ} : v.isValue = true → Step P (.scope k v, μ) (v, μ)
  | scopeError {k m μ} : Step P (.scope k (.error m), μ) (.error m, μ)
  /-- an abort unwinds like an error, but `try` does not catch it (warp throws it with its own wasm tag) -/
  | escape {F : Frame} {ev k v μ} : F.ready = true → v.isValue = true → Step P (F.plug (.abort ev k v), μ) (.abort ev k v, μ)
  | tryAbort {ev k v h μ} : v.isValue = true → Step P (.tryCatch (.abort ev k v) h, μ) (.abort ev k v, μ)
  /-- leaving the handler's scope fixes the depth of the block whose handler ran: the handlers outside it -/
  | scopeAbort {j ev k v μ} : v.isValue = true → Step P (.scope j (.abort ev k v), μ) (.abort ev (some (k.getD j)) v, μ)
  /-- the block of ev at that depth ends with v; any other block passes the abort on -/
  | forNil {y b v μ} : v.isValue = true → Step P (.forIn y .nil b v, μ) (v, μ)
  | forCons {y h t b v μ} : h.isValue = true → t.isValue = true → v.isValue = true →
      Step P (.forIn y (.cons h t) b v, μ) (.forIn y t b (b.subst y h), μ)
  | forText {y s b v μ} : v.isValue = true → Step P (.forIn y (.text s) b v, μ) (walkText y s b v, μ)
  | forRef {y a t b v μ} : v.isValue = true → Step P (.forIn y (.lref a t) b v, μ) (.forIn y (μ.items (.lref a t)) b v, μ)
  | forOther {y l b v μ} : l.isValue = true → isList l = false → isText l = false → isShared l = false →
      v.isValue = true →
      Step P (.forIn y l b v, μ) (.error "not a list", μ)
  | handleAbort {ev' h ev k v μ} : v.isValue = true →
      Step P (.handle ev' h (.abort ev k v), μ) (if ev' = ev ∧ k = some μ.handlers.length then v else .abort ev k v, μ)

/-- every declared name has a cell that fits its mode and type -/
def CellOk (P : Program) (m : Mode) (t : Ty) : Option Cell → Prop
  | some .unset => m ≠ .charged
  | some (.val v) => m ≠ .charged ∧ v.isValue = true ∧ ∃ tv, HasType P Ctx.empty v tv ∧ sub tv t = true
  | some (.charged b) => m = .charged ∧ ∃ tb, HasType P Ctx.empty b tb ∧ sub tb t = true
  | none => False

/-- every field set on an instance holds a value of the field's type -/
def HeapOk (P : Program) (μ : Store) : Prop :=
  ∀ (a : Nat) (o : Obj), μ.heap[a]? = some o → ∀ f v, o.fields f = some v →
    v.isValue = true ∧ ∃ t tv, P.fieldTy o.path f = some t ∧ HasType P Ctx.empty v tv ∧ sub tv t = true

def HandlersOk (P : Program) (hs : List (String × Expr)) : Prop := ∀ p ∈ hs, HandlerOk P p.1 p.2

/-- every shared list's items are a list value whose items fit the list's element type -/
def ListsOk (P : Program) (ls : List (Ty × Expr)) : Prop :=
  ∀ (a : Nat) t items, ls[a]? = some (t, items) → items.isValue = true ∧ ∃ ti, HasType P Ctx.empty items ti ∧ sub ti (.list t) = true

def StoreOk (P : Program) (μ : Store) : Prop :=
  (∀ x m t, P.names x = some (m, t) → CellOk P m t (μ x)) ∧ HeapOk P μ ∧ HandlersOk P μ.handlers ∧ ListsOk P μ.lists

end Warp
