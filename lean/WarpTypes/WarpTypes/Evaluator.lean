import WarpTypes.Checker

/-! The executable semantics: `step` computes the next state (`step_sound`: every step it takes is a `Step`), `run`
iterates it with fuel (`run_sound`: the state it ends in is reachable by `Steps`). `outcome` runs an elaborated
program, so the differential test compares warp's values with the model's, not only the verdicts. -/

namespace Warp
open Ty Expr

/-- an abort whose value is computed: it unwinds -/
def Expr.aborting : Expr → Bool
  | .abort _ _ v => v.isValue
  | _ => false

theorem Expr.aborting_eq : ∀ {e : Expr}, e.aborting = true → ∃ ev k v, e = .abort ev k v ∧ v.isValue = true
  | .abort _ _ _, h => ⟨_, _, _, rfl, h⟩

/-- an abort leaving the scope of a handler with j handlers outside it -/
def Expr.rescope (j : Nat) : Expr → Expr
  | .abort ev k v => .abort ev (some (k.getD j)) v
  | e => e

/-- an abort reaching a block of ev' at depth d: its value if it is that block's, else it passes on -/
def Expr.landsAt (ev' : String) (d : Nat) : Expr → Expr
  | .abort ev k v => if ev' = ev ∧ k = some d then v else .abort ev k v
  | e => e

/-- a step inside frame F of its hole e, given e's own step r: an error or abort in the hole is raised -/
def stepIn (F : Frame) (e : Expr) (μ : Store) (r : Option (Expr × Store)) : Option (Expr × Store) :=
  match e with
  | .error m => some (.error m, μ)
  | _ => if e.aborting then some (e, μ) else r.map fun s => (F.plug s.1, s.2)

/-- two operands left to right, then `done` on their values -/
def stepPair (L : Expr → Frame) (R : Expr → Frame) (a b : Expr) (μ : Store) (ra rb : Option (Expr × Store))
    (done : Option (Expr × Store)) : Option (Expr × Store) :=
  if a.isValue then (if b.isValue then done else stepIn (R a) b μ rb) else stepIn (L b) a μ ra

def step (P : Program) (μ : Store) : Expr → Option (Expr × Store)
  | .cons h t => stepPair .consL .consR h t μ (step P μ h) (step P μ t) none
  | .glob x =>
    match μ x with
    | some (.val v) => some (v, μ)
    | some .unset => some (.error "unset", μ)
    | some (.charged b) => some (b, μ)
    | none => none
  | .add a b => stepPair .addL .addR a b μ (step P μ a) (step P μ b) (some (addValues a b, μ))
  | .arith op a b => stepPair (.arithL op) (.arithR op) a b μ (step P μ a) (step P μ b) (some (arithValues op a b, μ))
  | .lt a b => stepPair .ltL .ltR a b μ (step P μ a) (step P μ b) (some (ltValues a b, μ))
  | .eq s a b => stepPair (.eqL s) (.eqR s) a b μ (step P μ a) (step P μ b) (some (.bool (eqValues P μ s a b), μ))
  | .ite c a b => if c.isValue then some (if truthy c then a else b, μ) else stepIn (.ite a b) c μ (step P μ c)
  | .loop c b => some (.ite c (.seq b (.loop c b)) .unit, μ)
  | .seq a b => if a.isValue then some (b, μ) else stepIn (.seq b) a μ (step P μ a)
  | .index l i => stepPair .indexL .indexR l i μ (step P μ l) (step P μ i)
      (some ((nth l ((asInt i).getD 0)).getD (.error "index out of range"), μ))
  | .range a b => stepPair .rangeL .rangeR a b μ (step P μ a) (step P μ b) (some (rangeValues a b, μ))
  | .append a b => stepPair .appendL .appendR a b μ (step P μ a) (step P μ b) (some (appendValues a b, μ))
  | .assign x e => if e.isValue then some (e, μ.set x (.val e)) else stepIn (.assign x) e μ (step P μ e)
  | .init x e => if e.isValue then some (e, μ.set x (.val e)) else stepIn (.init x) e μ (step P μ e)
  | .letIn y t e b => if e.isValue then some (b.subst y e, μ) else stepIn (.letIn y t b) e μ (step P μ e)
  | .call f e =>
    if e.isValue then (P.funs f).map fun fn => (.letIn fn.param fn.paramTy e fn.body, μ)
    else stepIn (.call f) e μ (step P μ e)
  | .tryCatch e h =>
    match e with
    | .error _ => some (h, μ)
    | _ =>
      if e.aborting then some (e, μ)
      else if e.isValue then some (e, μ) else (step P μ e).map fun s => (.tryCatch s.1 h, s.2)
  | .cast e ts =>
    if e.isValue then some (if ts.any (fits e) then e else .error "type mismatch", μ) else stepIn (.cast ts) e μ (step P μ e)
  | .broadcast f e =>
    if e.isValue then
      match e with
      | .nil => some (.nil, μ)
      | .cons h t => some (.cons (.call f h) (.broadcast f t), μ)
      | _ => none
    else stepIn (.broadcast f) e μ (step P μ e)
  | .new p => some (.ref μ.heap.length p, μ.alloc p)
  | .get o f => if o.isValue then some (readField μ o f, μ) else stepIn (.get f) o μ (step P μ o)
  | .set o f v => stepPair (fun v => .setL f v) (fun o => .setR o f) o v μ (step P μ o) (step P μ v) (some (writeField μ o f v))
  | .isA e c => if e.isValue then some (.bool (isInstance e c), μ) else stepIn (.isA c) e μ (step P μ e)
  | .handle ev h b =>
    match b with
    | .error m => some (.error m, μ)
    | _ =>
      if b.aborting then some (b.landsAt ev μ.handlers.length, μ)
      else if b.isValue then some (b, μ)
      else (step P (μ.push ev h) b).map fun s => (.handle ev h s.1, s.2.withHandlers μ.handlers)
  | .emit ev e =>
    if e.isValue then
      some (match lookupHandler μ.handlers ev with
        | some (h, k) => (.scope k (h.subst eventLocal e), μ)
        | none =>
          match P.handlers ev with
          | some h => (.scope 0 (h.subst eventLocal e), μ)
          | none => (.unit, μ))
    else stepIn (.emit ev) e μ (step P μ e)
  | .scope k e =>
    match e with
    | .error m => some (.error m, μ)
    | _ =>
      if e.aborting then some (e.rescope k, μ)
      else if e.isValue then some (e, μ)
      else (step P (μ.outer k) e).map fun s => (.scope k s.1, s.2.withHandlers μ.handlers)
  | .abort ev k e => if e.isValue then none else stepIn (.abort ev k) e μ (step P μ e)
  | .forIn y l b =>
    if l.isValue then
      some (match l with
        | .nil => .unit
        | .cons h t => .seq (b.subst y h) (.forIn y t b)
        | .text s => walkText y s b
        | _ => .error "not a list", μ)
    else stepIn (.forIn y b) l μ (step P μ l)
  | .lam y b => some (.clo y b, μ)
  | .app f a => stepPair .appL .appR f a μ (step P μ f) (step P μ a)
      (some (match f with
        | .clo y b => b.subst y a
        | _ => .error "not a function", μ))
  | _ => none

variable {P : Program}

theorem stepIn_sound {F : Frame} (hF : F.ready = true) {e μ r s'}
    (hr : ∀ s, r = some s → Step P (e, μ) s) (h : stepIn F e μ r = some s') : Step P (F.plug e, μ) s' := by
  unfold stepIn at h
  split at h
  · cases h; exact .raise hF
  · split at h
    · rename_i ha; cases h; obtain ⟨_, _, _, rfl, hv⟩ := Expr.aborting_eq ha; exact .escape hF hv
    simp only [Option.map_eq_some_iff] at h
    obtain ⟨⟨e', μ'⟩, hs, rfl⟩ := h
    exact .frame hF (hr _ hs)

theorem stepPair_sound {L R : Expr → Frame} {a b μ ra rb done s'}
    (hL : (L b).ready = true) (hR : a.isValue = true → (R a).ready = true)
    (pa : (L b).plug a = (R a).plug b)
    (ha : ∀ s, ra = some s → Step P (a, μ) s) (hb : ∀ s, rb = some s → Step P (b, μ) s)
    (hd : a.isValue = true → b.isValue = true → done = some s' → Step P ((L b).plug a, μ) s')
    (h : stepPair L R a b μ ra rb done = some s') : Step P ((L b).plug a, μ) s' := by
  unfold stepPair at h
  split at h
  · split at h
    · exact hd (by assumption) (by assumption) h
    · rw [pa]; exact stepIn_sound (hR (by assumption)) hb h
  · exact stepIn_sound hL ha h

/-- **The evaluator takes only steps of the semantics** -/
theorem step_sound : ∀ {e : Expr} {μ s'}, step P μ e = some s' → Step P (e, μ) s' := by
  intro e
  induction e with
  | cons h t ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := .consL) (R := .consR) rfl id rfl (fun _ => ih1) (fun _ => ih2)
      (fun _ _ hd => by cases hd) hs
  | glob x =>
    intro μ s' hs; simp only [step] at hs
    split at hs <;> cases hs
    · exact .readValue (by assumption)
    · exact .readUnset (by assumption)
    · exact .readCharged (by assumption)
  | add a b ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := .addL) (R := .addR) rfl id rfl (fun _ => ih1) (fun _ => ih2)
      (fun va vb hd => by cases hd; exact .add va vb) hs
  | arith op a b ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := .arithL op) (R := .arithR op) rfl id rfl (fun _ => ih1) (fun _ => ih2)
      (fun va vb hd => by cases hd; exact .arith va vb) hs
  | lt a b ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := .ltL) (R := .ltR) rfl id rfl (fun _ => ih1) (fun _ => ih2)
      (fun va vb hd => by cases hd; exact .lt va vb) hs
  | eq s a b ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := .eqL s) (R := .eqR s) rfl id rfl (fun _ => ih1) (fun _ => ih2)
      (fun va vb hd => by cases hd; exact .eq va vb) hs
  | ite c a b ih _ _ =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .ite (by assumption)
    · exact stepIn_sound (F := .ite a b) rfl (fun _ => ih) hs
  | loop => intro μ s' hs; simp only [step] at hs; cases hs; exact .loop
  | seq a b ih _ =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .seq (by assumption)
    · exact stepIn_sound (F := .seq b) rfl (fun _ => ih) hs
  | index l i ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := .indexL) (R := .indexR) rfl id rfl (fun _ => ih1) (fun _ => ih2)
      (fun va vb hd => by cases hd; exact .index va vb) hs
  | range a b ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := .rangeL) (R := .rangeR) rfl id rfl (fun _ => ih1) (fun _ => ih2)
      (fun va vb hd => by cases hd; exact .range va vb) hs
  | append a b ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := .appendL) (R := .appendR) rfl id rfl (fun _ => ih1) (fun _ => ih2)
      (fun va vb hd => by cases hd; exact .append va vb) hs
  | assign x e ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .assign (by assumption)
    · exact stepIn_sound (F := .assign x) rfl (fun _ => ih) hs
  | init x e ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .init (by assumption)
    · exact stepIn_sound (F := .init x) rfl (fun _ => ih) hs
  | letIn y t e b ih _ =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .letIn (by assumption)
    · exact stepIn_sound (F := .letIn y t b) rfl (fun _ => ih) hs
  | call f e ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · simp only [Option.map_eq_some_iff] at hs
      obtain ⟨fn, hf, rfl⟩ := hs
      exact .call (by assumption) hf
    · exact stepIn_sound (F := .call f) rfl (fun _ => ih) hs
  | tryCatch e h ih _ =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .tryError
    · split at hs
      · rename_i ha; cases hs; obtain ⟨_, _, _, rfl, hv⟩ := Expr.aborting_eq ha; exact .tryAbort hv
      split at hs
      · cases hs; exact .tryValue (by assumption)
      · simp only [Option.map_eq_some_iff] at hs
        obtain ⟨⟨e', μ'⟩, he, rfl⟩ := hs
        exact .tryStep (ih he)
  | cast e ts ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .cast (by assumption)
    · exact stepIn_sound (F := .cast ts) rfl (fun _ => ih) hs
  | broadcast f e ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · split at hs
      · cases hs; exact .broadcastNil
      · cases hs; rename_i hv; simp [isValue] at hv; exact .broadcastCons hv.1 hv.2
      · cases hs
    · exact stepIn_sound (F := .broadcast f) rfl (fun _ => ih) hs
  | new p => intro μ s' hs; simp only [step] at hs; cases hs; exact .new
  | get o f ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .get (by assumption)
    · exact stepIn_sound (F := .get f) rfl (fun _ => ih) hs
  | set o f v ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := fun v => .setL f v) (R := fun o => .setR o f) rfl id rfl (fun _ => ih1)
      (fun _ => ih2) (fun vo vv hd => by cases hd; exact .set vo vv) hs
  | isA e c ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .isA (by assumption)
    · exact stepIn_sound (F := .isA c) rfl (fun _ => ih) hs
  | handle ev h b _ ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .handleError
    · split at hs
      · rename_i ha; cases hs; obtain ⟨_, _, _, rfl, hv⟩ := Expr.aborting_eq ha; exact .handleAbort hv
      split at hs
      · cases hs; exact .handleValue (by assumption)
      · simp only [Option.map_eq_some_iff] at hs
        obtain ⟨⟨e', μ'⟩, he, rfl⟩ := hs
        exact .handleStep (ih he)
  | emit ev e ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · rename_i hv
      cases hs
      split
      · rename_i hl; exact .emitBlock hv hl
      · rename_i hl
        split
        · rename_i hp; exact .emitProgram hv hl hp
        · rename_i hp; exact .emitNone hv hl hp
    · exact stepIn_sound (F := .emit ev) rfl (fun _ => ih) hs
  | scope k e ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs; exact .scopeError
    · split at hs
      · rename_i ha; cases hs; obtain ⟨_, _, _, rfl, hv⟩ := Expr.aborting_eq ha; exact .scopeAbort hv
      split at hs
      · cases hs; exact .scopeValue (by assumption)
      · simp only [Option.map_eq_some_iff] at hs
        obtain ⟨⟨e', μ'⟩, he, rfl⟩ := hs
        exact .scopeStep (ih he)
  | abort ev k e ih =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · cases hs
    · exact stepIn_sound (F := .abort ev k) rfl (fun _ => ih) hs
  | forIn y l b ih _ =>
    intro μ s' hs; simp only [step] at hs
    split at hs
    · rename_i hv; cases hs
      split
      · exact .forNil
      · simp only [isValue, Bool.and_eq_true] at hv; exact .forCons hv.1 hv.2
      · exact .forText
      · exact .forOther hv (by cases l <;> simp_all [isList]) (by cases l <;> simp_all [isText])
    · exact stepIn_sound (F := .forIn y b) rfl (fun _ => ih) hs
  | lam y b => intro μ s' hs; simp only [step] at hs; cases hs; exact .lam
  | app f a ih1 ih2 =>
    intro μ s' hs
    exact stepPair_sound (L := .appL) (R := .appR) rfl id rfl (fun _ => ih1) (fun _ => ih2)
      (fun vf va hd => by
        cases hd
        split
        · exact .app va
        · exact .appOther vf va (by cases f <;> simp_all [isClosure])) hs
  | _ => intro μ s' hs; simp [step] at hs

/-- at most `fuel` steps -/
def run (P : Program) : Nat → Expr × Store → Expr × Store
  | 0, s => s
  | fuel + 1, s =>
    match step P s.2 s.1 with
    | some s' => run P fuel s'
    | none => s

theorem run_sound : ∀ (fuel : Nat) (s : Expr × Store), Steps P s (run P fuel s)
  | 0, s => .refl
  | fuel + 1, (e, μ) => by
    simp only [run]
    split
    · rename_i s' hs; exact .step (step_sound hs) (run_sound fuel s')
    · exact .refl

/-- a value as warp prints it; `?` where the model does not keep what warp prints (numbers, instances) -/
def display : Expr → String
  | .bool b => if b then "yes" else "no"
  | .int n => toString n
  | .text s => s!"\"{s}\""
  | .cons h t => "[" ++ " ".intercalate (showItems (.cons h t)) ++ "]"
  | _ => "?"
where showItems : Expr → List String
  | .cons h t => display h :: showItems t
  | _ => []

def FUEL : Nat := 10000

/-- what an elaborated program gives when run: `rejected`, `error`, a value as warp prints it, or `?` -/
def outcome (items : List Item) : String :=
  let s := elaborate items
  match s.breaksPlaced, s.check with
  | false, _ | _, none => "rejected"
  | true, some _ =>
    match (run s.program FUEL (s.main, s.store)).1 with
    | .error _ => "error"
    | v => if v.isValue then display v else "?"

end Warp
