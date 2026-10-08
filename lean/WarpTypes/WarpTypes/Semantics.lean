import WarpTypes.Typing

/-! Small-step semantics of W0: left to right, call by value. An `error` in any evaluation position propagates,
except under `try`. Main-level names are cells of the store; class instances live in its heap, which only grows. -/

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

def asNumber : Expr → Int
  | .num n => n
  | e => (asInt e).getD 0

/-- a value as `+` joins it to a text -/
def render : Expr → String
  | .text s => s
  | .bool b => if b then "true" else "false"
  | .int n | .num n => toString n
  | _ => ""

def isText : Expr → Bool
  | .text _ => true
  | _ => false

/-- a text's first character and the rest: `for c in "ab"` walks its one-character texts -/
def peel (s : String) : Option (String × String) :=
  match s.toList with
  | [] => none
  | c :: cs => some (c.toString, String.ofList cs)

/-- one step of a walk over a text: done at the end, else the body for the first character, then the rest -/
def walkText (y : String) (s : String) (b last : Expr) : Expr :=
  match peel s with
  | none => last
  | some (c, rest) => .forIn y (.text rest) b (b.subst y (.text c))

def isNumber : Expr → Bool
  | .bool _ | .int _ | .num _ => true
  | _ => false

def arithValues (op : ArithOp) (a b : Expr) : Expr :=
  if !(isNumber a && isNumber b) then .error "not a number" else
  if (op == .mod || op == .div) && asNumber b == 0 then .error "divide by zero" else
  match asInt a, asInt b with
  | some x, some y => if op == .div && x % y != 0 then .num (op.apply x y) else .int (op.apply x y)
  | _, _ => .num (op.apply (asNumber a) (asNumber b))

def isList : Expr → Bool
  | .nil | .cons _ _ => true
  | _ => false

def concat : Expr → Expr → Expr
  | .cons h t, b => .cons h (concat t b)
  | _, b => b

/-- `+`: two lists concatenate (`xs + ys`), numbers add, a text side makes a text -/
def addValues (a b : Expr) : Expr :=
  if isList a && isList b then concat a b else
  if !((isNumber a || isText a) && (isNumber b || isText b)) then .error "not addable" else
  if isText a || isText b then .text (render a ++ render b) else
  match asInt a, asInt b with
  | some x, some y => .int (x + y)
  | _, _ => .num (asNumber a + asNumber b)

/-- numbers by value, texts in codepoint order (`"a" < "b"`) -/
def ltValues (a b : Expr) : Expr :=
  if isNumber a && isNumber b then .bool (decide (asNumber a < asNumber b)) else
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

/-- false, 0, "", ø and [] are falsy -/
def truthy : Expr → Bool
  | .bool b => b
  | .int n => n != 0
  | .num n => n != 0
  | .text s => !s.isEmpty
  | .unit | .nil => false
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
  | .num _ => some .number
  | .text _ => some .text
  | .unit => some .unit
  | .nil => some (.list .never)
  | .ref _ p => some (.cls p)
  | .lref _ t => some (.list t)
  | .cons h t =>
    match valueType h, valueType t with
    | some a, some l =>
      match element l with
      | some e => some (.list (join a e))
      | none => none
    | _, _ => none
  | _ => none

/-- the run-time type test of a cast -/
def fits (v : Expr) (t : Ty) : Bool :=
  match valueType v with
  | some tv => sub tv t
  | none => false

/-- `xs.add(v)` of values: v joins the shared list if it fits the list's element type -/
def pushValues (μ : Store) : Expr → Expr → Expr × Store
  | .lref a t, v =>
    match μ.listAt a t with
    | some items => if fits v t then (.lref a t, μ.writeList a (concat items (.cons v .nil))) else (.error "type mismatch", μ)
    | none => (.error "dangling list", μ)
  | _, _ => (.error "not a list", μ)

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
  | broadcast (f : String)
  | get (f : String)
  | setL (f : String) (v : Expr) | setR (o : Expr) (f : String)
  | isA (c : String)
  | emit (ev : String)
  | abort (ev : String) (k : Option Nat)
  | forIn (y : String) (b last : Expr)
  | loopLast (c b : Expr)
  | forInLast (y : String) (l b : Expr)
  | appL (a : Expr) | appR (f : Expr)
  | share (t : Ty)
  | pushL (v : Expr) | pushR (l : Expr)

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
  | broadcast f, e => .broadcast f e
  | get f, e => .get e f
  | setL f v, e => .set e f v
  | setR o f, e => .set o f e
  | isA c, e => .isA e c
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

/-- a right position needs the left operand evaluated -/
def ready : Frame → Bool
  | consR h | addR h | arithR _ h | ltR h | eqR _ h | indexR h | rangeR h | appendR h | appR h | setR h _ | pushR h
  | forInLast _ h _ => h.isValue
  | _ => true

end Frame

inductive Step (P : Program) : Expr × Store → Expr × Store → Prop where
  | frame {F : Frame} {e e' μ μ'} : F.ready = true → Step P (e, μ) (e', μ') → Step P (F.plug e, μ) (F.plug e', μ')
  | raise {F : Frame} {m μ} : F.ready = true → Step P (F.plug (.error m), μ) (.error m, μ)
  | tryStep {e e' h μ μ'} : Step P (e, μ) (e', μ') → Step P (.tryCatch e h, μ) (.tryCatch e' h, μ')
  | tryError {m h μ} : Step P (.tryCatch (.error m) h, μ) (h, μ)
  | tryValue {v h μ} : v.isValue = true → Step P (.tryCatch v h, μ) (v, μ)
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
      Step P (.index l i, μ) ((nth (μ.items l) ((asInt i).getD 0)).getD (.error "index out of range"), μ)
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
  | cast {v ts μ} : v.isValue = true → Step P (.cast v ts, μ) (if ts.any (fits v) then v else .error "type mismatch", μ)
  | new {p μ} : Step P (.new p, μ) (.ref μ.heap.length p, μ.alloc p)
  | get {o f μ} : o.isValue = true → Step P (.get o f, μ) (readField μ o f, μ)
  | set {o f v μ} : o.isValue = true → v.isValue = true → Step P (.set o f v, μ) (writeField μ o f v)
  | isA {v c μ} : v.isValue = true → Step P (.isA v c, μ) (.bool (isInstance v c), μ)
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
