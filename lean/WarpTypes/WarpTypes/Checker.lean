import WarpTypes.Soundness

/-! The executable checker: `typeOf` computes the type `HasType` gives (`typeOf_sound`), and `Spec.check` elaborates a
whole program in source order and checks it, so an accepted program is safe (`check_safe`).
This is what the differential test runs on programs exported from warp (src/law/type_model.rs). -/

namespace Warp
open Ty Expr

/-- the checker reads a field only from an error, a class declaring it, or a dynamic value (P201: `s: Shape; s.r` is
a compile error, `s = Circle(…); s.r` is checked when it runs) -/
def strictRead (P : Program) : Ty → String → Bool
  | .never, _ | .any, _ => true
  | .cls p, f => (P.fieldTy p f).isSome
  | _, _ => false

/-- the checker takes `#` and `++` of a list or of a dynamic value (checked when it runs) -/
def listy (t : Ty) : Bool := t == .any || (element t).isSome

/-- what a call takes statically: a function, an error, or a dynamic value (checked when it runs) -/
def callable : Ty → Bool
  | .fn _ | .never | .any => true
  | _ => false

/-- a whole number, or a value of unknown type (checked when it runs) -/
def wholeNumber (t : Ty) : Bool := sub t .int || t == .any

/-- what `*` repeats statically: a text times a whole number, in either order (P1) -/
def repeats (a b : Ty) : Bool := (textual a && wholeNumber b) || (wholeNumber a && textual b)


/-- a value written into a typed list that evidently does not fit its element type: warp checks literal items when it
compiles (checks.rs misfit_item: `xs: [int16] = [70000]`, `xs.add("a")`); any other value is checked when it runs -/
def evidentMisfit (v : Expr) (t : Ty) : Bool := v.isValue && !fits v t

/-- what `as` converts statically: anything to text or bool, numbers and texts to a number type, else what a cast
admits (`[1] as int` is refused) -/
def convertible (te : Ty) : Ty → Bool
  | .text | .bool => true
  | .int | .exact | .number => consub te .number || consub te .text
  | t => consub te t

/-- a quantity meets only its own dimensions (or a dynamic value, or an error): `1 m + 1 s`, `1 m == 1`, `1 m < "a"`
and `if c { 1 m } else { 2 }` are DimensionErrors when warp compiles them (src/units/static_units.rs) -/
def dimensionsAgree (a b : Ty) : Bool :=
  !(a.isQuantity || b.isQuantity) || a == b || a == .any || b == .any || a == .never || b == .never

/-- `-` takes one dimension, `*` and `/` combine quantities and numbers, `%` and `^` take no quantity -/
def dimensionsCombine (op : ArithOp) (a b : Ty) : Bool :=
  if op = .sub then dimensionsAgree a b
  else !(a.isQuantity || b.isQuantity) || a == .any || b == .any || (op == .mul || op == .div) && a.dimsOf.isSome && b.dimsOf.isSome

/-- what `-`, `*`, `/` and `<` take: numbers, quantities and a dynamic value -/
def measurable (t : Ty) : Bool := numeric t || t.isQuantity

def typeOf (P : Program) (Γ : Ctx) : Expr → Option Ty
  | .bool _ => some .bool
  | .int _ => some .int
  | .num _ => some .exact
  | .flt _ => some .number
  | .fail _ => some .any
  | .failed e => (typeOf P Γ e).map fun _ => .bool
  | .qty _ d => some (.quantity d)
  | .text s => some (textTy s)
  | .unit => some .unit
  | .nil => some (.list .never)
  | .cons h t =>
    match typeOf P Γ h, typeOf P Γ t with
    | some a, some l => (element l).map fun e => .list (join a e)
    | _, _ => none
  | .glob x => (P.names x).map Prod.snd
  | .loc y => Γ y
  | .add a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb =>
      if dimensionsAgree ta tb && ((addable ta && addable tb) || (listy ta && listy tb) || ta.isQuantity) then some (plus ta tb)
      else none
    | _, _ => none
  | .arith op a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb =>
      if dimensionsCombine op ta tb && ((measurable ta && measurable tb) || (op == .mul && repeats ta tb)) then some (op.ty ta tb)
      else none
    | _, _ => none
  | .lt a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb =>
      if dimensionsAgree ta tb && ((measurable ta && measurable tb) || (textual ta && textual tb)) then some .bool else none
    | _, _ => none
  | .eq _ a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb => if dimensionsAgree ta tb then some .bool else none
    | _, _ => none
  | .ite c a b =>
    match typeOf P Γ c, typeOf P Γ a, typeOf P Γ b with
    | some _, some ta, some tb => if dimensionsAgree ta tb then some (join ta tb) else none
    | _, _, _ => none
  | .loop c b d =>
    match typeOf P Γ c, typeOf P Γ b, typeOf P Γ d with
    | some _, some tb, some td => some (join tb (join td .unit))
    | _, _, _ => none
  | .seq a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some _, some tb => some tb
    | _, _ => none
  -- a number index is checked when it runs: `xs[n/2]`
  | .index l i =>
    match typeOf P Γ l, typeOf P Γ i with
    | some tl, some ti => if listy tl || tl.isText then (if consub ti .number then some (elementTy tl) else none) else none
    | _, _ => none
  | .range a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb => if consub ta .number && consub tb .number then some (.list (arithTy ta tb)) else none
    | _, _ => none
  | .append a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb => if listy ta && listy tb then some (.list (join (listElem ta) (listElem tb))) else none
    | _, _ => none
  | .assign x e =>
    match P.names x, typeOf P Γ e with
    | some (.var, t), some te => if sub te t then some t else none
    | _, _ => none
  | .init x e =>
    match P.names x, typeOf P Γ e with
    | some (m, t), some te => if m != .charged && sub te t then some t else none
    | _, _ => none
  | .letIn y t e b =>
    match typeOf P Γ e with
    | some te => if sub te t then typeOf P (Γ.set y t) b else none
    | none => none
  | .call f e =>
    match P.funs f, typeOf P Γ e with
    | some fd, some te => if sub te fd.paramTy then some fd.result else none
    | _, _ => none
  | .error _ => some .never
  | .tryCatch e h =>
    match typeOf P Γ e, typeOf P Γ h with
    | some te, some th => some (join te th)
    | _, _ => none
  | .cast e ts => (typeOf P Γ e).bind fun te => if ts.any (consub te) then some (joinAll ts) else none
  | .conv e t => (typeOf P Γ e).bind fun te => if convertible te t then some t else none
  | .broadcast f e =>
    match P.funs f, typeOf P Γ e with
    | some fd, some te =>
      match element te with
      | some a => if sub a fd.paramTy then some (.list fd.result) else none
      | none => none
    | _, _ => none
  | .ref _ p | .new p => some (.cls p)
  | .lref _ t => some (.list t)
  /- statically, a list and a write that may fit; each write is checked again when it runs -/
  | .share e t => (typeOf P Γ e).bind fun te => if consub te (.list t) && !evidentMisfit e (.list t) then some (.list t) else none
  | .push l v =>
    match typeOf P Γ l, typeOf P Γ v with
    | some tl, some tv => if listy tl && consub tv (listElem tl) && !evidentMisfit v (listElem tl) then some tl else none
    | _, _ => none
  | .setAt l i v =>
    match typeOf P Γ l, typeOf P Γ i, typeOf P Γ v with
    | some tl, some ti, some tv =>
      if listy tl && consub ti .number && consub tv (listElem tl) && !evidentMisfit v (listElem tl) then some tv else none
    | _, _, _ => none
  | .get e f => (typeOf P Γ e).bind fun te => if strictRead P te f then some (P.readTy te f) else none
  | .set e f v =>
    match typeOf P Γ e, typeOf P Γ v with
    | some te, some tv => (P.writeTy te f).bind fun t => if sub tv t then some tv else none
    | _, _ => none
  | .isA e _ => (typeOf P Γ e).map fun _ => .bool
  | .handle ev h b =>
    match P.effects ev, typeOf P (Γ.set eventLocal .any) h, typeOf P Γ b with
    | some R, some th, some tb => if sub th R then some (join tb (P.aborts ev)) else none
    | _, _, _ => none
  | .emit ev e =>
    match P.effects ev, typeOf P Γ e with
    | some R, some _ => some (join R .unit)
    | _, _ => none
  | .scope _ e => typeOf P Γ e
  | .abort ev _ e => (typeOf P Γ e).bind fun te => if sub te (P.aborts ev) then some .never else none
  | .forIn y l b d => (typeOf P Γ l).bind fun tl =>
    if listy tl || tl.isText then (typeOf P (Γ.set y (elementTy tl)) b).bind fun tb =>
      (typeOf P Γ d).map fun td => join tb (join td .unit)
    else none
  | .lam y b => (typeOf P (Γ.set y .any) b).map .fn
  | .clo y b => (typeOf P (Ctx.empty.set y .any) b).map .fn
  | .app f a =>
    match typeOf P Γ f, typeOf P Γ a with
    | some tf, some _ => if callable tf then some (resultTy tf) else none
    | _, _ => none

theorem typeOf_sound {P : Program} : ∀ {e : Expr} {Γ t}, typeOf P Γ e = some t → HasType P Γ e t := by
  intro e
  induction e with
  | text s => intro Γ t h; simp [typeOf] at h; subst h; exact .ofText s
  | bool | int | num | flt | qty | unit | nil | fail => intro Γ t h; simp [typeOf] at h; subst h; constructor
  | cons h tl ih1 ih2 =>
    intro Γ t hs; simp only [typeOf] at hs; split at hs
    · rename_i a l ha hl
      simp only [Option.map_eq_some_iff] at hs
      obtain ⟨e, he, rfl⟩ := hs
      exact .cons (ih1 ha) (ih2 hl) he
    · cases hs
  | glob x =>
    intro Γ t h; simp only [typeOf, Option.map_eq_some_iff] at h
    obtain ⟨⟨m, t'⟩, hx, rfl⟩ := h
    exact .glob hx
  | loc y => intro Γ t h; exact .loc h
  | add a b ih1 ih2 =>
    intro Γ t h
    cases ha : typeOf P Γ a <;> cases hb : typeOf P Γ b <;> simp only [typeOf, ha, hb] at h <;> try cases h
    split at h
    · cases h; exact .add (ih1 ha) (ih2 hb)
    · cases h
  | arith op a b ih1 ih2 =>
    intro Γ t h
    cases ha : typeOf P Γ a <;> cases hb : typeOf P Γ b <;> simp only [typeOf, ha, hb] at h <;> try cases h
    split at h
    · cases h; exact .arith (ih1 ha) (ih2 hb)
    · cases h
  | lt a b ih1 ih2 =>
    intro Γ t h
    cases ha : typeOf P Γ a <;> cases hb : typeOf P Γ b <;> simp only [typeOf, ha, hb] at h <;> try cases h
    split at h
    · cases h; exact .lt (ih1 ha) (ih2 hb)
    · cases h
  | eq s a b ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i ha hb; split at h
      · cases h; exact .eq (ih1 ha) (ih2 hb)
      · cases h
    · cases h
  | ite c a b ih0 ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i hc ha hb; split at h
      · cases h; exact .ite (ih0 hc) (ih1 ha) (ih2 hb)
      · cases h
    · cases h
  | loop c b d ih1 ih2 ih3 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i hc hb hd; cases h; exact .loop (ih1 hc) (ih2 hb) (ih3 hd)
    · cases h
  | seq a b ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i ha hb; cases h; exact .seq (ih1 ha) (ih2 hb)
    · cases h
  | index l i ih1 ih2 =>
    intro Γ t h
    cases hl : typeOf P Γ l <;> cases hi : typeOf P Γ i <;> simp only [typeOf, hl, hi] at h <;> try cases h
    split at h
    · split at h
      · cases h; exact .index (ih1 hl) (ih2 hi)
      · cases h
    · cases h
  | range a b ih1 ih2 =>
    intro Γ t h
    cases ha : typeOf P Γ a <;> cases hb : typeOf P Γ b <;> simp only [typeOf, ha, hb] at h <;> try cases h
    split at h
    · cases h; exact .range (ih1 ha) (ih2 hb)
    · cases h
  | append a b ih1 ih2 =>
    intro Γ t h
    cases ha : typeOf P Γ a <;> cases hb : typeOf P Γ b <;> simp only [typeOf, ha, hb] at h <;> try cases h
    split at h
    · cases h; exact .append (ih1 ha) (ih2 hb)
    · cases h
  | assign x e ih =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · split at h
      · rename_i hx he hs; cases h; exact .assign hx (ih he) hs
      · cases h
    · cases h
  | init x e ih =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · split at h
      · rename_i hx he hc; cases h; simp at hc; exact .init hx hc.1 (ih he) hc.2
      · cases h
    · cases h
  | letIn y t' e b ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · split at h
      · rename_i he hs; exact .letIn (ih1 he) hs (ih2 h)
      · cases h
    · cases h
  | call f e ih =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · split at h
      · rename_i hf he hs; cases h; exact .call hf (ih he) hs
      · cases h
    · cases h
  | error => intro Γ t h; simp [typeOf] at h; subst h; constructor
  | tryCatch e h' ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i he hh; cases h; exact .tryCatch (ih1 he) (ih2 hh)
    · cases h
  | cast e ts ih =>
    intro Γ t h; simp only [typeOf, Option.bind_eq_some_iff] at h
    obtain ⟨_, he, hc⟩ := h
    split at hc
    · cases hc; exact .cast (ih he)
    · cases hc
  | conv e t ih =>
    intro Γ t' h; simp only [typeOf, Option.bind_eq_some_iff] at h
    obtain ⟨_, he, hc⟩ := h
    split at hc
    · cases hc; exact .conv (ih he)
    · cases hc
  | broadcast f e ih =>
    intro Γ t h
    cases hf : P.funs f <;> cases he : typeOf P Γ e <;> simp only [typeOf, hf, he] at h <;> try cases h
    split at h
    · split at h
      · rename_i hel hs; cases h; exact .broadcast hf (ih he) hel hs
      · cases h
    · cases h
  | ref | new | lref => intro Γ t h; simp [typeOf] at h; subst h; constructor
  | share e t ih =>
    intro Γ t' h; simp only [typeOf, Option.bind_eq_some_iff] at h
    obtain ⟨_, he, hc⟩ := h
    split at hc
    · cases hc; exact .share (ih he)
    · cases hc
  | push l v ih1 ih2 =>
    intro Γ t h
    cases hl : typeOf P Γ l <;> cases hv : typeOf P Γ v <;> simp only [typeOf, hl, hv] at h <;> try cases h
    split at h
    · cases h; exact .push (ih1 hl) (ih2 hv)
    · cases h
  | setAt l i v ih1 ih2 ih3 =>
    intro Γ t h
    cases hl : typeOf P Γ l <;> cases hi : typeOf P Γ i <;> cases hv : typeOf P Γ v <;>
      simp only [typeOf, hl, hi, hv] at h <;> try cases h
    split at h
    · cases h; exact .setAt (ih1 hl) (ih2 hi) (ih3 hv)
    · cases h
  | get e f ih =>
    intro Γ t h; simp only [typeOf, Option.bind_eq_some_iff] at h
    obtain ⟨te, he, hr⟩ := h
    split at hr
    · cases hr; exact .get (ih he)
    · cases hr
  | set e f v ih1 ih2 =>
    intro Γ t h
    cases he : typeOf P Γ e <;> cases hv : typeOf P Γ v <;> simp only [typeOf, he, hv] at h <;> try cases h
    simp only [Option.bind_eq_some_iff] at h
    obtain ⟨tf, hw, h⟩ := h
    split at h
    · rename_i hs; cases h; exact .set (ih1 he) hw (ih2 hv) hs
    · cases h
  | isA e c ih =>
    intro Γ t h; simp only [typeOf, Option.map_eq_some_iff] at h
    obtain ⟨_, he, rfl⟩ := h
    exact .isA (ih he)
  | failed e ih =>
    intro Γ t h; simp only [typeOf, Option.map_eq_some_iff] at h
    obtain ⟨_, he, rfl⟩ := h
    exact .failed (ih he)
  | handle ev h b ih1 ih2 =>
    intro Γ t hs
    cases hR : P.effects ev <;> cases hh : typeOf P (Γ.set eventLocal .any) h <;> cases hb : typeOf P Γ b <;>
      simp only [typeOf, hR, hh, hb] at hs <;> try cases hs
    split at hs
    · rename_i hsub; cases hs; exact .handle hR (ih1 hh) hsub (ih2 hb)
    · cases hs
  | emit ev e ih =>
    intro Γ t hs
    cases hR : P.effects ev <;> cases he : typeOf P Γ e <;> simp only [typeOf, hR, he] at hs <;> try cases hs
    exact .emit hR (ih he)
  | scope k e ih => intro Γ t hs; exact .scope (ih hs)
  | forIn y l b d ih1 ih2 ih3 =>
    intro Γ t hs
    simp only [typeOf, Option.bind_eq_some_iff] at hs
    obtain ⟨tl, hl, hs⟩ := hs
    split at hs
    · simp only [Option.bind_eq_some_iff, Option.map_eq_some_iff] at hs
      obtain ⟨tb, hb, td, hd, rfl⟩ := hs
      exact .forIn (ih1 hl) (sub_refl _) (ih2 hb) (ih3 hd)
    · cases hs
  | lam y b ih =>
    intro Γ t hs; simp only [typeOf, Option.map_eq_some_iff] at hs
    obtain ⟨_, hb, rfl⟩ := hs; exact .lam (ih hb)
  | clo y b ih =>
    intro Γ t hs; simp only [typeOf, Option.map_eq_some_iff] at hs
    obtain ⟨_, hb, rfl⟩ := hs; exact .clo (ih hb)
  | app f a ih1 ih2 =>
    intro Γ t h
    cases hf : typeOf P Γ f <;> cases ha : typeOf P Γ a <;> simp only [typeOf, hf, ha] at h <;> try cases h
    split at h
    · cases h; exact .app (ih1 hf) (ih2 ha)
    · cases h
  | abort ev k e ih =>
    intro Γ t hs
    simp only [typeOf, Option.bind_eq_some_iff] at hs
    obtain ⟨te, he, hs⟩ := hs
    split at hs
    · rename_i st; cases hs; exact .abort (ih he) st
    · cases hs

/-! ## Whole programs, as warp writes them: top-level items in source order -/

/-- a main-level name and how it was declared -/
structure Decl where
  name : String
  mode : Mode
  type : Ty
  /-- the body of a charged name -/
  charged : Option Expr := none
  isGlobal : Bool := false
  /-- declared with a type (`x: int = 1`), not inferred from its values -/
  annotated : Bool := false

/-- a checked program: declarations, functions and the main expression, after elaboration -/
structure Spec where
  decls : List Decl
  funs : List (String × Fn)
  main : Expr
  /-- each class with the fields it declares itself -/
  classes : List (String × List (String × Ty)) := []
  /-- each event's result type; an event no handler answers gives `never` -/
  effects : List (String × Ty) := []
  /-- the program-wide handlers `on ev {…}` -/
  handlers : List (String × Expr) := []
  /-- each event's abort type: the join of what its handlers' `break`s give -/
  aborts : List (String × Ty) := []

def Spec.program (s : Spec) : Program where
  names x := (s.decls.find? (·.name == x)).map fun d => (d.mode, d.type)
  funs f := (s.funs.find? (·.1 == f)).map (·.2)
  globals x := (s.decls.find? (·.name == x)).any (·.isGlobal)
  fields c f := (s.classes.find? (·.1 == c)).bind fun (_, fields) => fields.lookup f
  effects ev := some ((s.effects.lookup ev).getD .never)
  handlers ev := (s.handlers.find? (·.1 == ev)).map (·.2)
  aborts ev := (s.aborts.lookup ev).getD .never
  fieldNames p := p.flatMap fun c => ((s.classes.lookup c).getD []).map (·.1)

/-- the store before main runs: charged names hold their body, the others are unset -/
def Spec.store (s : Spec) : Store where
  vars x := (s.decls.find? (·.name == x)).map fun d =>
    match d.charged with
    | some b => .charged b
    | none => .unset

def Decl.ok (P : Program) (d : Decl) : Bool :=
  match d.mode, d.charged with
  | .charged, some b => (typeOf P Ctx.empty b).any (sub · d.type)
  | .charged, none => false
  | _, some _ => false
  | _, none => true

def Fn.ok (P : Program) (fn : Fn) : Bool :=
  (typeOf P (Ctx.empty.set fn.param fn.paramTy) fn.body).any (sub · fn.result) &&
    fn.body.assigned.all P.globals

def handlerOk (P : Program) (ev : String) (h : Expr) : Bool :=
  match P.effects ev, typeOf P (Ctx.empty.set eventLocal .any) h with
  | some R, some th => sub th R
  | _, _ => false

def Spec.check (s : Spec) : Option Ty :=
  let P := s.program
  if s.decls.all (Decl.ok P) && s.funs.all (fun f => f.2.ok P) && s.handlers.all (fun (ev, h) => handlerOk P ev h) then
    typeOf P Ctx.empty s.main
  else none

theorem Spec.check_sound {s : Spec} {t} (h : s.check = some t) :
    ProgramOk s.program ∧ StoreOk s.program s.store ∧ HasType s.program Ctx.empty s.main t := by
  unfold Spec.check at h
  dsimp only at h
  split at h
  · rename_i hok
    simp only [Bool.and_eq_true, List.all_eq_true] at hok
    obtain ⟨⟨hdecls, hfuns⟩, hhandlers⟩ := hok
    refine ⟨⟨?_, ?_⟩, ⟨?_, fun a o ho => by simp [Spec.store] at ho, fun p hp => by simp [Spec.store] at hp,
      fun a t xs hx => by simp [Spec.store] at hx⟩,
      typeOf_sound h⟩
    · intro f fn hf
      simp only [Spec.program, Option.map_eq_some_iff] at hf
      obtain ⟨⟨g, fn'⟩, hfind, rfl⟩ := hf
      have hfn := hfuns _ (List.mem_of_find?_eq_some hfind)
      simp only [Fn.ok, Bool.and_eq_true, Option.any_eq_true, List.all_eq_true] at hfn
      obtain ⟨⟨tb, htb, hs⟩, hg⟩ := hfn
      exact ⟨⟨tb, typeOf_sound htb, hs⟩, hg⟩
    · intro ev hd hfound
      simp only [Spec.program, Option.map_eq_some_iff] at hfound
      obtain ⟨⟨ev', hd'⟩, hfind, rfl⟩ := hfound
      have hok := hhandlers _ (List.mem_of_find?_eq_some hfind)
      have hev : ev' = ev := by simpa using List.find?_some hfind
      subst hev
      unfold handlerOk at hok
      split at hok
      · rename_i R th hR hth; exact ⟨R, th, hR, typeOf_sound hth, hok⟩
      · cases hok
    · intro x m t' hx
      simp only [Spec.program, Option.map_eq_some_iff] at hx
      obtain ⟨d, hfind, hd⟩ := hx
      cases hd
      have hdok := hdecls _ (List.mem_of_find?_eq_some hfind)
      simp only [Spec.store, hfind, Option.map_some]
      unfold Decl.ok at hdok
      split at hdok
      · rename_i h1 h2
        simp only [Option.any_eq_true] at hdok
        obtain ⟨tb, htb, hs⟩ := hdok
        simp only [CellOk, h1, h2]
        first | exact ⟨tb, typeOf_sound htb, hs⟩ | exact ⟨trivial, tb, typeOf_sound htb, hs⟩
      · cases hdok
      · cases hdok
      · rename_i h1 h2
        simp only [CellOk, h1]
        first | exact h1 | exact h2
  · cases h

/-- **An accepted program is safe**: every state it reaches is well typed, and done, failed, or able to step -/
theorem check_safe {s : Spec} {t} (h : s.check = some t) {s'} (hs : Steps s.program (s.main, s.store) s') :
    (∃ t', HasType s.program Ctx.empty s'.1 t' ∧ sub t' t = true) ∧ StoreOk s.program s'.2 ∧
      Progresses s.program s'.2 s'.1 :=
  let ⟨hP, hμ, ht⟩ := Spec.check_sound h
  safety hP hs ht hμ

/-! ## Elaboration: warp's source order, the types warp infers -/

/-- one top-level item of a warp program -/
inductive Item where
  /-- `x = e`, `x: T = e`, `const x = e`, `global x = e`: the first binding of a main-level name -/
  | bind (name : String) (mode : Mode) (annotation : Option Ty) (value : Expr) (isGlobal : Bool)
  /-- `z := e` -/
  | charged (name : String) (body : Expr)
  /-- `f(y: T) := body` (warp hoists functions); an unannotated parameter (`none`) takes the values it is given -/
  | function (name param : String) (paramTy : Option Ty) (body : Expr)
  /-- `class C : Parent { f: T … }` and the variants of `type Color = red | rgb(r: int, …)`: the fields C declares
  itself, an unannotated one (`none`) typed by the values written to it; its ancestor chain is written into the
  types that name it -/
  | classDef (name : String) (fields : List (String × Option Ty))
  /-- the cell a local of a function lives in, a fresh instance per call (warp's locals change, W0's are bound by
  substitution): a class of the one field `cellField`, typed like a main-level name by the first value written, or
  by its annotation (an annotated parameter assigned in the body, P203: annotations are strict) -/
  | cell (name : String) (declared : Option Ty)
  /-- `on ev {h}` at main level: a program-wide handler -/
  | on (ev : String) (h : Expr)
  /-- any other statement -/
  | statement (e : Expr)

/-- a name given no annotation holds the type of its first value; a list stays a list of anything (list-element-types:
an undeclared list may take items of any type); a codepoint is a text (`x = "a"; x = "bc"`) -/
def widen : Ty → Ty
  | .list _ => .list .any
  | .codepoint => .text
  | t => t

/-- the values a main-level name is given -/
def valuesOf (x : String) : Expr → List Expr
  | .assign y e | .init y e => (if y = x then [e] else []) ++ valuesOf x e
  | .cons a b | .add a b | .arith _ a b | .lt a b | .eq _ a b | .seq a b | .index a b | .range a b | .append a b
  | .tryCatch a b | .app a b => valuesOf x a ++ valuesOf x b
  | .ite c a b | .loop c a b | .forIn _ c a b => valuesOf x c ++ valuesOf x a ++ valuesOf x b
  | .letIn _ _ e b => valuesOf x e ++ valuesOf x b
  | .set a _ b => valuesOf x a ++ valuesOf x b
  | .call _ e | .cast e _ | .conv e _ | .broadcast _ e | .get e _ | .isA e _ | .failed e | .emit _ e | .scope _ e | .abort _ _ e => valuesOf x e
  | .handle _ h b => valuesOf x h ++ valuesOf x b
  | .lam _ b => valuesOf x b
  | _ => []

/-- P45: a name declared by its first value widens over the numbers it is given (`x = 1; x = 2.5` is a number);
a value of another kind (`x = 1; x = "a"`) leaves the type, so its assignment is rejected -/
def widenOver (s : Spec) (d : Decl) : Decl :=
  if d.annotated || d.mode == .charged || d.type == .bool then d else
  let widened := (valuesOf d.name s.main).foldl (init := d.type) fun t value =>
    match typeOf s.program Ctx.empty value with
    | some tv => if sub (join t tv) .number then join t tv else t
    | none => t
  { d with type := widened }

/-- rewrites an expression bottom-up, `f` seeing each node with its children rewritten and the locals in scope -/
def Expr.rewrite (P : Program) (f : Ctx → Expr → Expr) (Γ : Ctx) : Expr → Expr
  | .call g e => f Γ (.call g (e.rewrite P f Γ))
  | .cons a b => f Γ (.cons (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .add a b => f Γ (.add (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .arith op a b => f Γ (.arith op (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .lt a b => f Γ (.lt (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .eq s a b => f Γ (.eq s (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .ite c a b => f Γ (.ite (c.rewrite P f Γ) (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .loop c b d => f Γ (.loop (c.rewrite P f Γ) (b.rewrite P f Γ) (d.rewrite P f Γ))
  | .seq a b => f Γ (.seq (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .index a b => f Γ (.index (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .range a b => f Γ (.range (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .append a b => f Γ (.append (a.rewrite P f Γ) (b.rewrite P f Γ))
  | .assign x e => f Γ (.assign x (e.rewrite P f Γ))
  | .init x e => f Γ (.init x (e.rewrite P f Γ))
  | .letIn y t e b => f Γ (.letIn y t (e.rewrite P f Γ) (b.rewrite P f (Γ.set y t)))
  | .tryCatch e h => f Γ (.tryCatch (e.rewrite P f Γ) (h.rewrite P f Γ))
  | .cast e ts => f Γ (.cast (e.rewrite P f Γ) ts)
  | .conv e t => f Γ (.conv (e.rewrite P f Γ) t)
  | .get e g => f Γ (.get (e.rewrite P f Γ) g)
  | .set e g v => f Γ (.set (e.rewrite P f Γ) g (v.rewrite P f Γ))
  | .isA e c => f Γ (.isA (e.rewrite P f Γ) c)
  | .failed e => f Γ (.failed (e.rewrite P f Γ))
  | .handle ev h b => f Γ (.handle ev (h.rewrite P f (Γ.set eventLocal .any)) (b.rewrite P f Γ))
  | .emit ev e => f Γ (.emit ev (e.rewrite P f Γ))
  | .scope k e => f Γ (.scope k (e.rewrite P f Γ))
  | .abort ev k e => f Γ (.abort ev k (e.rewrite P f Γ))
  | .forIn y l b d =>
    let l := l.rewrite P f Γ
    f Γ (.forIn y l (b.rewrite P f (Γ.set y (((typeOf P Γ l).map elementTy).getD .any))) (d.rewrite P f Γ))
  | .lam y b => f Γ (.lam y (b.rewrite P f (Γ.set y .any)))
  | .app a b => f Γ (.app (a.rewrite P f Γ) (b.rewrite P f Γ))
  | e => f Γ e

/-- warp decides broadcasting at compile time: a call whose argument is a list of what the function takes -/
def resolveCalls (P : Program) : Ctx → Expr → Expr :=
  Expr.rewrite P fun Γ e => match e with
    | .call f e =>
      match P.funs f, typeOf P Γ e with
      | some fd, some te =>
        if sub te fd.paramTy then .call f e
        else if (element te).any (sub · fd.paramTy) then .broadcast f e else .call f e
      | _, _ => .call f e
    | e => e

/-- gradual typing: a value of static type any given to a narrower name or parameter is checked when it runs
(`level = event.level` is `level = cast event.level [int]`); warp admits these at compile time -/
def castDynamicValues (P : Program) : Ctx → Expr → Expr :=
  Expr.rewrite P fun Γ e =>
    let checked (place : Option Ty) (v : Expr) (give : Expr → Expr) :=
      match place, typeOf P Γ v with
      | some t, some .any => if t == .any then e else give (.cast v [t])
      | _, _ => e
    match e with
    | .assign x v => checked ((P.names x).map (·.2)) v (.assign x)
    | .init x v => checked ((P.names x).map (·.2)) v (.init x)
    | .call f v => checked ((P.funs f).map (·.paramTy)) v (.call f)
    | .set o f v => checked ((typeOf P Γ o).bind (P.writeTy · f)) v (.set o f)
    | e => e

/-- the function with a provisional result; its calls resolved (a recursive call sees that result) and its values of
unknown type checked where they go to a typed place (so its result is inferred from the body that runs) -/
def draftFunction (s : Spec) (name param : String) (paramTy result : Ty) (body : Expr) : Fn :=
  let draft : Fn := { param, paramTy, result, body }
  let s' := { s with funs := (name, draft) :: s.funs }
  let Γ := Ctx.empty.set param paramTy
  { draft with body := castDynamicValues s'.program Γ (resolveCalls s'.program Γ body) }

/-- a function's result: iterate from `never` so a recursive call sees its own result type -/
def inferFunction (s : Spec) (name param : String) (paramTy : Ty) (body : Expr) : Nat → Ty → Fn
  | 0, r => draftFunction s name param paramTy r body
  | n + 1, r =>
    let fn := draftFunction s name param paramTy r body
    let s' := { s with funs := (name, fn) :: s.funs }
    match typeOf s'.program (Ctx.empty.set param paramTy) fn.body with
    | some r' => if r' == r then fn else inferFunction s name param paramTy body n (join r r')
    | none => fn

/-- statements in order; the last one gives the program's value -/
def sequence : List Expr → Expr
  | [] => .unit
  | [e] => e
  | e :: rest => .seq e (sequence rest)

def RESULT_ROUNDS : Nat := 4

/-- all functions' results, inferred together so a function may call one defined after it, or call each other (warp
hoists functions): each round infers every function against the drafts of the round before -/
def inferFunctions (s : Spec) (functions : List (String × String × Ty × Expr)) : Nat → List (String × Fn) → List (String × Fn)
  | 0, drafts => drafts
  | n + 1, drafts =>
    let s' := { s with funs := drafts }
    inferFunctions s functions n (functions.map fun (f, y, t, b) =>
      (f, inferFunction s' f y t b RESULT_ROUNDS (((drafts.lookup f).map (·.result)).getD .never)))

/-- the field of a cell (`Item.cell`) -/
def cellField : String := "·value"

/-- the types guessed for unannotated parameters (by function) and fields (by class and field), and each event's
result type: the join of what its handlers give -/
structure Guesses where
  params : List (String × Ty) := []
  /-- each cell's first value's type, from the latest round -/
  cells : List (String × Ty) := []
  fields : List ((String × String) × Ty) := []
  effects : List (String × Ty) := []
  aborts : List (String × Ty) := []

/-- the list with t joined into the entry of key -/
def joinAt {α} [BEq α] (entries : List (α × Ty)) (key : α) (t : Ty) : List (α × Ty) :=
  (key, join ((entries.lookup key).getD .never) t) :: entries.filter (·.1 != key)

/-- a value given to an unannotated place: an argument of f, or a write to field f of a class chain -/
inductive Observation where
  | argument (f : String) (t : Ty)
  | field (path : List String) (f : String) (t : Ty)
  /-- a handler of ev giving t -/
  | handler (ev : String) (t : Ty)
  /-- a `break` of a handler of ev giving t -/
  | abort (ev : String) (t : Ty)

/-- the arguments and field writes of an expression, with their types -/
def observe (P : Program) (Γ : Ctx) : Expr → List Observation
  | .call f e => (typeOf P Γ e).toList.map (.argument f) ++ observe P Γ e
  | .set o f v =>
    (match typeOf P Γ o, typeOf P Γ v with
      | some (.cls p), some tv => [.field p f tv]
      | _, _ => []) ++ observe P Γ o ++ observe P Γ v
  | .letIn y t e b => observe P Γ e ++ observe P (Γ.set y t) b
  | .cons a b | .add a b | .arith _ a b | .lt a b | .eq _ a b | .seq a b | .index a b | .range a b | .append a b
  | .tryCatch a b | .app a b => observe P Γ a ++ observe P Γ b
  | .ite c a b | .loop c a b => observe P Γ c ++ observe P Γ a ++ observe P Γ b
  | .assign _ e | .init _ e | .cast e _ | .conv e _ | .broadcast _ e | .get e _ | .isA e _ | .failed e | .emit _ e | .scope _ e => observe P Γ e
  | .handle ev h b => observeHandler P Γ ev h ++ observe P Γ b
  | .abort ev _ e => ((typeOf P Γ e).getD .any |> Observation.abort ev) :: observe P Γ e
  | .forIn y l b d => observe P Γ l ++ observe P (Γ.set y (((typeOf P Γ l).map elementTy).getD .any)) b ++ observe P Γ d
  | .lam y b => observe P (Γ.set y .any) b
  | _ => []
where
  observeHandler (P : Program) (Γ : Ctx) (ev : String) (h : Expr) : List Observation :=
    let Γh := Γ.set eventLocal .any
    ((typeOf P Γh h).getD .any |> Observation.handler ev) :: observe P Γh h

/-- the items with every unannotated place typed by its guess; one never given a value holds anything -/
def Guesses.fill (g : Guesses) : Item → Item
  | .function f y none b => .function f y (some (((g.params.lookup f).filter (· != .never)).getD .any)) b
  | .classDef c fields =>
    .classDef c (fields.map fun (f, t) => (f, some (t.getD (((g.fields.lookup (c, f)).filter (· != .never)).getD .any))))
  | .cell c (some t) => .classDef c [(cellField, some t)]
  | .cell c none => .classDef c [(cellField, some (((g.cells.lookup c).filter (· != .never)).map widen |>.getD .any))]
  | item => item

def Guesses.add (g : Guesses) (classes : List (String × List (String × Option Ty))) : Observation → Guesses
  | .argument f t => { g with params := joinAt g.params f t }
  | .field p f t =>
    -- the class of the chain that declares f
    match p.find? fun c => ((classes.lookup c).getD []).any (·.1 == f) with
    | some c =>
      { g with fields := joinAt g.fields (c, f) t }
    | none => g
  | .handler ev t => { g with effects := joinAt g.effects ev t }
  | .abort ev t => { g with aborts := joinAt g.aborts ev t }

/-- one elaboration pass; functions are inferred seeing the main-level names `known` from the pass before (a function
may read a main-level name declared after it: warp hoists functions) -/
def elaboratePass (items : List Item) (effects aborts : List (String × Ty)) (known : List Decl) : Spec :=
  let functions := items.filterMap fun | .function f y t b => some (f, y, t.getD .any, b) | _ => none
  let rest := items.filter fun | .function .. | .classDef .. | .cell .. => false | _ => true
  let classes := items.filterMap fun | .classDef c fields => some (c, fields.map fun (f, t) => (f, t.getD .any)) | _ => none
  let base : Spec := { decls := known, funs := [], main := .unit, classes, effects, aborts }
  let drafts := functions.map fun (f, y, t, b) => (f, ({ param := y, paramTy := t, result := .never, body := b } : Fn))
  let withFunctions := { base with funs := inferFunctions base functions RESULT_ROUNDS drafts }
  let withFunctions := { withFunctions with decls := [] }
  let step (s : Spec) (statements : List Expr) : Item → Spec × List Expr
    | .bind x m annotation value g =>
      let value := resolveCalls s.program Ctx.empty value
      let t := annotation.getD (widen ((typeOf s.program Ctx.empty value).getD .any))
      ({ s with decls := s.decls ++ [{ name := x, mode := m, type := t, isGlobal := g, annotated := annotation.isSome }] },
        statements ++ [.init x value])
    | .charged x body =>
      let body := resolveCalls s.program Ctx.empty body
      let t := (typeOf s.program Ctx.empty body).getD .any
      ({ s with decls := s.decls ++ [{ name := x, mode := .charged, type := t, charged := some body }] }, statements)
    | .statement e => (s, statements ++ [resolveCalls s.program Ctx.empty e])
    | .on ev h => ({ s with handlers := s.handlers ++ [(ev, resolveCalls s.program (Ctx.empty.set eventLocal .any) h)] }, statements)
    | .function .. | .classDef .. | .cell .. => (s, statements)
  let (s, statements) := rest.foldl (init := (withFunctions, [])) fun (s, st) item => step s st item
  let s := { s with main := sequence statements }
  let s := (List.range RESULT_ROUNDS).foldl (init := s) fun s _ => { s with decls := s.decls.map (widenOver s) }
  let lax := castDynamicValues s.program
  { s with
    main := lax Ctx.empty s.main
    funs := s.funs.map fun (f, fd) => (f, { fd with body := lax (Ctx.empty.set fd.param fd.paramTy) fd.body })
    handlers := s.handlers.map fun (ev, h) => (ev, lax (Ctx.empty.set eventLocal .any) h) }

def elaborateTyped (items : List Item) (effects aborts : List (String × Ty)) : Spec :=
  elaboratePass items effects aborts (elaboratePass items effects aborts []).decls

/-- one round of inference: elaborate with the guesses so far, then join in the values the program gives -/
def Guesses.refine (items : List Item) (g : Guesses) : Guesses :=
  let s := elaborateTyped (items.map g.fill) g.effects g.aborts
  let classes := items.filterMap fun | .classDef c fields => some (c, fields) | _ => none
  let cells := items.filterMap fun | .cell c none => some c | _ => none
  let bodies := s.funs.map fun (_, fd) => observe s.program (Ctx.empty.set fd.param fd.paramTy) fd.body
  let handlers := s.handlers.map fun (ev, h) => observe.observeHandler s.program Ctx.empty ev h
  let observed := observe s.program Ctx.empty s.main ++ bodies.flatten ++ handlers.flatten
  -- a local holds the type of its first value, widened over the numbers it is given, as a main-level name (P45)
  let firsts := observed.foldl (init := ([] : List (String × Ty))) fun firsts o => match o with
    | .field [c] f t =>
      if f != cellField || !cells.contains c then firsts else
      match firsts.lookup c with
      | none => firsts ++ [(c, t)]
      | some first => if sub (join first t) .number then firsts.map fun (d, u) => (d, if d == c then join u t else u) else firsts
    | _ => firsts
  { observed.foldl (fun g o => g.add classes o) g with cells := firsts }

/-- warp's elaboration: unannotated parameters and fields are typed by the values the program gives them (P173: a
parameter given several kinds holds anything), then the program is elaborated in source order -/
def elaborate (items : List Item) : Spec :=
  let guesses := (List.range RESULT_ROUNDS).foldl (init := ({} : Guesses)) fun g _ => g.refine items
  elaborateTyped (items.map guesses.fill) guesses.effects guesses.aborts

/-- every `break` sits in a block handler of its own event, outside loops (a `break` in a loop is the loop's, which
W0 does not have): so no abort escapes its block -/
def Expr.breaksIn (ev : Option String) : Expr → Bool
  | .abort e k x => ev == some e && k.isNone && x.breaksIn ev
  | .handle e h b => h.breaksIn (some e) && b.breaksIn ev
  | .loop c b d | .forIn _ c b d => c.breaksIn none && b.breaksIn none && d.breaksIn none
  | .lam _ b => b.breaksIn none
  | .letIn _ _ e b => e.breaksIn ev && b.breaksIn ev
  | .cons a b | .add a b | .arith _ a b | .lt a b | .eq _ a b | .seq a b | .index a b | .range a b | .append a b | .tryCatch a b
  | .set a _ b | .app a b => a.breaksIn ev && b.breaksIn ev
  | .ite c a b => c.breaksIn ev && a.breaksIn ev && b.breaksIn ev
  | .assign _ e | .init _ e | .call _ e | .cast e _ | .conv e _ | .broadcast _ e | .get e _ | .isA e _ | .failed e | .emit _ e
  | .scope _ e => e.breaksIn ev
  | _ => true

def Spec.breaksPlaced (s : Spec) : Bool :=
  s.main.breaksIn none && s.funs.all (·.2.body.breaksIn none) && s.handlers.all (·.2.breaksIn none) &&
    s.decls.all fun d => (d.charged.map (Expr.breaksIn none)).getD true

/-- the verdict line the differential test reads: `ok <type>` or `rejected` -/
def verdict (items : List Item) : String :=
  let s := elaborate items
  match s.breaksPlaced, s.check with
  | true, some t => s!"ok {t.name}"
  | _, _ => "rejected"

end Warp
