import WarpTypes.Lemmas

/-! Progress and preservation for W0, and type safety: a well-typed program never gets stuck. -/

namespace Warp
open Ty Expr

variable {P : Program}

/-- a loop's type after a round whose value has a type below the body's -/
theorem loop_bound {tb t td : Ty} (h : sub t tb = true) : sub (join tb (join t .any)) (join tb (join td .any)) = true :=
  join_least (join_upper_left _ _)
    (join_least (sub_trans h (join_upper_left _ _)) (sub_trans (join_upper_right td .any) (join_upper_right tb _)))

/-- the type of a plugged frame: the hole has a type, and anything of a smaller type in the hole gives a smaller type -/
theorem frame_typing {Γ} (F : Frame) {e t} (h : HasType P Γ (F.plug e) t) :
    ∃ te, HasType P Γ e te ∧
      ∀ {e' te'}, HasType P Γ e' te' → sub te' te = true → ∃ t', HasType P Γ (F.plug e') t' ∧ sub t' t = true := by
  cases F <;> simp only [Frame.plug] at h ⊢
  case consL =>
    cases h with | cons hh ht he =>
    exact ⟨_, hh, fun h' s => ⟨_, .cons h' ht he, by simpa using join_mono s (sub_refl _)⟩⟩
  case consR =>
    cases h with | cons hh ht he =>
    refine ⟨_, ht, fun h' s => ?_⟩
    obtain ⟨e', he', se⟩ := element_mono s he
    exact ⟨_, .cons hh h' he', by simpa using join_mono (sub_refl _) se⟩
  case addL => cases h with | add ha hb => exact ⟨_, ha, fun h' s => ⟨_, .add h' hb, plus_mono s (sub_refl _)⟩⟩
  case addR => cases h with | add ha hb => exact ⟨_, hb, fun h' s => ⟨_, .add ha h', plus_mono (sub_refl _) s⟩⟩
  case arithL => cases h with | arith ha hb => exact ⟨_, ha, fun h' s => ⟨_, .arith h' hb, ArithOp.ty_mono _ s (sub_refl _)⟩⟩
  case arithR => cases h with | arith ha hb => exact ⟨_, hb, fun h' s => ⟨_, .arith ha h', ArithOp.ty_mono _ (sub_refl _) s⟩⟩
  case ltL => cases h with | lt ha hb => exact ⟨_, ha, fun h' _ => ⟨_, .lt h' hb, sub_refl _⟩⟩
  case ltR => cases h with | lt ha hb => exact ⟨_, hb, fun h' _ => ⟨_, .lt ha h', sub_refl _⟩⟩
  case eqL => cases h with | eq ha hb => exact ⟨_, ha, fun h' _ => ⟨_, .eq h' hb, sub_refl _⟩⟩
  case eqR => cases h with | eq ha hb => exact ⟨_, hb, fun h' _ => ⟨_, .eq ha h', sub_refl _⟩⟩
  case ite => cases h with | ite hc ha hb => exact ⟨_, hc, fun h' _ => ⟨_, .ite h' ha hb, sub_refl _⟩⟩
  case seq => cases h with | seq ha hb => exact ⟨_, ha, fun h' _ => ⟨_, .seq h' hb, sub_refl _⟩⟩
  case indexL => cases h with | index hl hi => exact ⟨_, hl, fun h' s => ⟨_, .index h' hi, elementTy_mono s⟩⟩
  case indexR => cases h with | index hl hi => exact ⟨_, hi, fun h' _ => ⟨_, .index hl h', sub_refl _⟩⟩
  case appL => cases h with | app hf ha => exact ⟨_, hf, fun h' s => ⟨_, .app h' ha, resultTy_mono s⟩⟩
  case appR => cases h with | app hf ha => exact ⟨_, ha, fun h' _ => ⟨_, .app hf h', sub_refl _⟩⟩
  case rangeL => cases h with | range ha hb => exact ⟨_, ha, fun h' s => ⟨_, .range h' hb, by simpa [sub] using arithTy_mono s (sub_refl _)⟩⟩
  case rangeR => cases h with | range ha hb => exact ⟨_, hb, fun h' s => ⟨_, .range ha h', by simpa [sub] using arithTy_mono (sub_refl _) s⟩⟩
  case appendL =>
    cases h with | append ha hb =>
    exact ⟨_, ha, fun h' s => ⟨_, .append h' hb, by simpa using join_mono (listElem_mono s) (sub_refl _)⟩⟩
  case appendR =>
    cases h with | append ha hb =>
    exact ⟨_, hb, fun h' s => ⟨_, .append ha h', by simpa using join_mono (sub_refl _) (listElem_mono s)⟩⟩
  case assign =>
    cases h with | assign hx he st => exact ⟨_, he, fun h' s => ⟨_, .assign hx h' (sub_trans s st), sub_refl _⟩⟩
  case init =>
    cases h with | init hx hm he st => exact ⟨_, he, fun h' s => ⟨_, .init hx hm h' (sub_trans s st), sub_refl _⟩⟩
  case letIn =>
    cases h with | letIn he st hb => exact ⟨_, he, fun h' s => ⟨_, .letIn h' (sub_trans s st) hb, sub_refl _⟩⟩
  case call =>
    cases h with | call hf he st => exact ⟨_, he, fun h' s => ⟨_, .call hf h' (sub_trans s st), sub_refl _⟩⟩
  case cast => cases h with | cast he => exact ⟨_, he, fun h' _ => ⟨_, .cast h', sub_refl _⟩⟩
  case conv => cases h with | conv he => exact ⟨_, he, fun h' _ => ⟨_, .conv h', sub_refl _⟩⟩
  case broadcast =>
    cases h with | broadcast hf he hel ha =>
    refine ⟨_, he, fun h' s => ?_⟩
    obtain ⟨a', he', sa⟩ := element_mono s hel
    exact ⟨_, .broadcast hf h' he' (sub_trans sa ha), sub_refl _⟩
  case get => cases h with | get he => exact ⟨_, he, fun h' s => ⟨_, .get h', readTy_mono s _⟩⟩
  case setL =>
    cases h with | set he hw hv st =>
    refine ⟨_, he, fun h' s => ?_⟩
    obtain ⟨t', hw', st'⟩ := writeTy_anti s hw
    exact ⟨_, .set h' hw' hv (sub_trans st st'), sub_refl _⟩
  case setR =>
    cases h with | set he hw hv st =>
    exact ⟨_, hv, fun h' s => ⟨_, .set he hw h' (sub_trans s st), s⟩⟩
  case isA => cases h with | isA he => exact ⟨_, he, fun h' _ => ⟨_, .isA h', sub_refl _⟩⟩
  case failed => cases h with | failed he => exact ⟨_, he, fun h' _ => ⟨_, .failed h', sub_refl _⟩⟩
  case orElse => cases h with | orElse ha hb => exact ⟨_, ha, fun h' s => ⟨_, .orElse h' hb, join_mono (strip_mono s) (sub_refl _)⟩⟩
  case emit => cases h with | emit hR he => exact ⟨_, he, fun h' _ => ⟨_, .emit hR h', sub_refl _⟩⟩
  case abort => cases h with | abort he st => exact ⟨_, he, fun h' s => ⟨_, .abort h' (sub_trans s st), sub_refl _⟩⟩
  case share => cases h with | share he => exact ⟨_, he, fun h' _ => ⟨_, .share h', sub_refl _⟩⟩
  case pushL => cases h with | push hl hv => exact ⟨_, hl, fun h' s => ⟨_, .push h' hv, s⟩⟩
  case pushR => cases h with | push hl hv => exact ⟨_, hv, fun h' _ => ⟨_, .push hl h', sub_refl _⟩⟩
  case setAtL => cases h with | setAt hl hi hv => exact ⟨_, hl, fun h' _ => ⟨_, .setAt h' hi hv, sub_refl _⟩⟩
  case setAtI => cases h with | setAt hl hi hv => exact ⟨_, hi, fun h' _ => ⟨_, .setAt hl h' hv, sub_refl _⟩⟩
  case setAtR => cases h with | setAt hl hi hv => exact ⟨_, hv, fun h' s => ⟨_, .setAt hl hi h', s⟩⟩
  case forIn =>
    cases h with
    | forIn hl sT hb hd => exact ⟨_, hl, fun h' s => ⟨_, .forIn h' (sub_trans (elementTy_mono s) sT) hb hd, sub_refl _⟩⟩
  case loopLast =>
    cases h with
    | loop hc hb hd => exact ⟨_, hd, fun h' s => ⟨_, .loop hc hb h', join_mono (sub_refl _) (join_mono s (sub_refl _))⟩⟩
  case forInLast =>
    cases h with
    | forIn hl sT hb hd => exact ⟨_, hd, fun h' s => ⟨_, .forIn hl sT hb h', join_mono (sub_refl _) (join_mono s (sub_refl _))⟩⟩

/-- a value of a class type is a reference -/
theorem cls_value {Γ v p} (h : HasType P Γ v (.cls p)) (hv : v.isValue = true) : ∃ a, v = .ref a p := by
  cases v <;> simp [isValue] at hv <;> (try cases h) <;> exact ⟨_, rfl⟩

theorem HeapOk.alloc {μ : Store} (h : HeapOk P μ) (p : List String) : HeapOk P (μ.alloc p) := by
  intro a o ho f v hf
  simp only [Store.alloc, List.getElem?_append] at ho
  split at ho
  · exact h a o ho f v hf
  · rw [List.getElem?_singleton] at ho; split at ho
    · cases ho; simp at hf
    · cases ho

theorem HeapOk.write {μ : Store} (h : HeapOk P μ) {a p o f v t tv} (ho : μ.obj a p = some o)
    (hf : P.fieldTy p f = some t) (hv : v.isValue = true) (htv : HasType P Ctx.empty v tv) (st : sub tv t = true) :
    HeapOk P (μ.write a f v) := by
  simp only [Store.obj, Option.filter_eq_some_iff, beq_iff_eq] at ho
  obtain ⟨ha, rfl⟩ := ho
  intro b o' hb g w hg
  simp only [Store.write, List.getElem?_modify] at hb
  by_cases hab : a = b
  · subst hab; rw [ha] at hb; cases hb
    simp only at hg ⊢
    by_cases hgf : g = f
    · subst hgf; simp at hg; subst hg; exact ⟨hv, t, tv, hf, htv, st⟩
    · simp [hgf] at hg; exact h a o ha g w hg
  · simp [hab] at hb; exact h b o' hb g w hg

theorem readField_typed {μ : Store} (hμ : HeapOk P μ) {o p f t} (ho : HasType P Ctx.empty o (.cls p))
    (vo : o.isValue = true) (hf : P.fieldTy p f = some t) :
    ∃ t', HasType P Ctx.empty (readField μ o f) t' ∧ sub t' t = true := by
  obtain ⟨a, rfl⟩ := cls_value ho vo
  simp only [readField]
  split
  · rename_i obj hobj
    simp only [Store.obj, Option.filter_eq_some_iff, beq_iff_eq] at hobj
    obtain ⟨ha, rfl⟩ := hobj
    cases hg : obj.fields f with
    | none => exact ⟨_, .error, sub_never _⟩
    | some v =>
      obtain ⟨_, t', tv, hf', htv, st⟩ := hμ a obj ha f v hg
      rw [hf] at hf'; cases hf'
      exact ⟨tv, by simpa using htv, st⟩
  · exact ⟨_, .error, sub_never _⟩

/-- reading a field of anything gives a typed expression: a field value of the heap or an error -/
theorem readField_any {μ : Store} (hμ : HeapOk P μ) (o : Expr) (f : String) :
    ∃ t', HasType P Ctx.empty (readField μ o f) t' := by
  unfold readField
  split
  · split
    · rename_i obj hobj
      simp only [Store.obj, Option.filter_eq_some_iff] at hobj
      cases hg : obj.fields f with
      | none => exact ⟨_, .error⟩
      | some v =>
        obtain ⟨_, _, tv, _, htv, _⟩ := hμ _ obj hobj.1 f v hg
        exact ⟨tv, by simpa using htv⟩
    · exact ⟨_, .error⟩
  · exact ⟨_, .error⟩

/-- `o.f` of a value o: below the field type when the static class declares f, typed anyway (`any`) otherwise -/
theorem get_typed {μ : Store} (hμ : HeapOk P μ) {o te f} (ho : HasType P Ctx.empty o te) (vo : o.isValue = true) :
    ∃ t', HasType P Ctx.empty (readField μ o f) t' ∧ sub t' (P.readTy te f) = true := by
  have dynamic : P.readTy te f = .any → ∃ t', HasType P Ctx.empty (readField μ o f) t' ∧ sub t' (P.readTy te f) = true :=
    fun h => (readField_any hμ o f).imp fun _ h' => ⟨h', by simp [h]⟩
  cases te with
  | never => cases o <;> simp [isValue] at vo <;> cases ho
  | cls p =>
    cases hf : P.fieldTy p f with
    | some t => simpa [Program.readTy, hf] using readField_typed hμ ho vo hf
    | none => exact dynamic (by simp [Program.readTy, hf])
  | _ => exact dynamic rfl

theorem concat_value {b : Expr} (vb : b.isValue = true) : ∀ {a : Expr}, a.isValue = true → (concat a b).isValue = true := by
  intro a
  induction a <;> intro va <;> simp_all [concat, isValue]

/-- what a list operation reads of a value: a value of a smaller type -/
theorem items_typed {μ : Store} (hl : ListsOk P μ.lists) {v tv} (h : HasType P Ctx.empty v tv) (hv : v.isValue = true) :
    (μ.items v).isValue = true ∧ ∃ t', HasType P Ctx.empty (μ.items v) t' ∧ sub t' tv = true := by
  cases v with
  | lref a t =>
    cases h
    cases hx : μ.listAt a t with
    | none => simp only [Store.items, hx, Option.getD]; exact ⟨rfl, _, .nil, by simp⟩
    | some items =>
      simp only [Store.items, hx, Option.getD]
      simp only [Store.listAt, Option.map_eq_some_iff, Option.filter_eq_some_iff, beq_iff_eq] at hx
      obtain ⟨⟨t0, xs⟩, ⟨hget, ht0⟩, rfl⟩ := hx
      simp only at ht0; subst ht0
      exact hl a _ _ hget
  | _ => exact ⟨hv, _, h, sub_refl _⟩

theorem ListsOk.alloc {ls : List (Ty × Expr)} (h : ListsOk P ls) {t items ti} (hv : items.isValue = true)
    (ht : HasType P Ctx.empty items ti) (st : sub ti (.list t) = true) : ListsOk P (ls ++ [(t, items)]) := by
  intro a t' xs hx
  rw [List.getElem?_append] at hx
  split at hx
  · exact h a t' xs hx
  · rw [List.getElem?_singleton] at hx; split at hx
    · cases hx; exact ⟨hv, ti, ht, st⟩
    · cases hx

theorem ListsOk.write {ls : List (Ty × Expr)} (h : ListsOk P ls) {a t old items ti} (ha : ls[a]? = some (t, old))
    (hv : items.isValue = true) (ht : HasType P Ctx.empty items ti) (st : sub ti (.list t) = true) :
    ListsOk P (ls.modify a fun c => (c.1, items)) := by
  intro b t' xs hb
  by_cases hab : a = b
  · subst hab; simp [ha] at hb; obtain ⟨rfl, rfl⟩ := hb; exact ⟨hv, ti, ht, st⟩
  · simp [hab] at hb; exact h b t' xs hb

/-- `xs.add(v)`: the shared list itself, its new item checked against its element type, or an error -/
theorem push_typed {μ : Store} (hμ : StoreOk P μ) {l v tl tv} (hl : HasType P Ctx.empty l tl) (vl : l.isValue = true)
    (_hv : HasType P Ctx.empty v tv) (vv : v.isValue = true) :
    (∃ t', HasType P Ctx.empty (pushValues μ l v).1 t' ∧ sub t' tl = true) ∧ StoreOk P (pushValues μ l v).2 := by
  cases l with
  | lref a t =>
    cases hl
    simp only [pushValues]
    split
    · rename_i items hx
      split
      · rename_i hfit
        simp only [Store.listAt, Option.map_eq_some_iff, Option.filter_eq_some_iff, beq_iff_eq] at hx
        obtain ⟨⟨t0, xs⟩, ⟨hget, ht0⟩, rfl⟩ := hx
        simp only at ht0; subst ht0
        obtain ⟨vi, ti, hti, sti⟩ := hμ.2.2.2 a _ _ hget
        obtain ⟨tv', hv', stv⟩ := fits_typed (P := P) (Γ := Ctx.empty) hfit
        obtain ⟨ei, hei, sei⟩ := element_mono sti rfl
        obtain ⟨tc, htc, stc⟩ := concat_typed (.cons hv' .nil rfl) (by simp [isValue, vv]) rfl vi hti hei
        refine ⟨⟨_, .lref, sub_refl _⟩, hμ.1, hμ.2.1, hμ.2.2.1, hμ.2.2.2.write hget
          (concat_value (by simp [isValue, vv]) vi) htc (sub_trans stc ?_)⟩
        simp only [sub_list]
        exact join_least sei (join_least stv (sub_never _))
      · exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
    · exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | _ => exact ⟨⟨_, .error, sub_never _⟩, hμ⟩

/-- `xs#i = v`: v, written into the shared list if it fits its element type, or an error -/
theorem setAt_typed {μ : Store} (hμ : StoreOk P μ) {l i v tv} (hv : HasType P Ctx.empty v tv) (vv : v.isValue = true) :
    (∃ t', HasType P Ctx.empty (setAtValues P μ l i v).1 t' ∧ sub t' tv = true) ∧ StoreOk P (setAtValues P μ l i v).2 := by
  cases l with
  | lref a t =>
    simp only [setAtValues]
    split
    · rename_i items hx
      split
      · rename_i hfit
        split
        · rename_i items' hr
          simp only [Store.listAt, Option.map_eq_some_iff, Option.filter_eq_some_iff, beq_iff_eq] at hx
          obtain ⟨⟨t0, xs⟩, ⟨hget, ht0⟩, rfl⟩ := hx
          simp only at ht0; subst ht0
          obtain ⟨vi, ti, hti, sti⟩ := hμ.2.2.2 a _ _ hget
          obtain ⟨tv', hv', stv⟩ := fits_typed (P := P) (Γ := Ctx.empty) hfit
          obtain ⟨vr, tr, htr, str⟩ := replaceAt_typed hv' vv stv vi hti sti hr
          exact ⟨⟨_, hv, sub_refl _⟩, hμ.1, hμ.2.1, hμ.2.2.1, hμ.2.2.2.write hget vr htr str⟩
        · exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
      · exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
    · exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | ref a p =>
    cases i with
    | text k =>
      simp only [setAtValues]
      split
      · rename_i t hf
        split
        · rename_i hc
          simp only [Bool.and_eq_true] at hc
          obtain ⟨o, ho⟩ := Option.isSome_iff_exists.1 hc.1
          obtain ⟨tv', hv', stv⟩ := fits_typed (P := P) (Γ := Ctx.empty) hc.2
          exact ⟨⟨_, hv, sub_refl _⟩, hμ.1, hμ.2.1.write ho hf vv hv' stv, hμ.2.2⟩
        · exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
      · exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
    | _ => exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | _ => exact ⟨⟨_, .error, sub_never _⟩, hμ⟩

theorem StoreOk.set {μ : Store} (hμ : StoreOk P μ) {x m t v tv} (hx : P.names x = some (m, t)) (hm : m ≠ .charged)
    (hv : v.isValue = true) (htv : HasType P Ctx.empty v tv) (st : sub tv t = true) :
    StoreOk P (μ.set x (.val v)) := by
  refine ⟨fun z mz tz hz => ?_, hμ.2⟩
  show CellOk P mz tz (if z = x then _ else _)
  split
  · subst_vars; rw [hx] at hz; cases hz; exact ⟨hm, hv, tv, htv, st⟩
  · exact hμ.1 z mz tz hz

theorem StoreOk.push {μ : Store} (hμ : StoreOk P μ) {ev h} (hh : HandlerOk P ev h) : StoreOk P (μ.push ev h) := by
  refine ⟨hμ.1, hμ.2.1, fun p hp => ?_, hμ.2.2.2⟩
  simp only [Store.push, Store.withHandlers, List.mem_cons] at hp
  rcases hp with rfl | hp
  · exact hh
  · exact hμ.2.2.1 p hp

theorem StoreOk.outer {μ : Store} (hμ : StoreOk P μ) (k : Nat) : StoreOk P (μ.outer k) :=
  ⟨hμ.1, hμ.2.1, fun p hp => hμ.2.2.1 p (List.mem_of_mem_drop hp), hμ.2.2.2⟩

/-- the handlers return to those of μ -/
theorem StoreOk.restore {μ μ' : Store} (hμ' : StoreOk P μ') (hμ : StoreOk P μ) : StoreOk P (μ'.withHandlers μ.handlers) :=
  ⟨hμ'.1, hμ'.2.1, hμ.2.2.1, hμ'.2.2.2⟩

/-- a handler run on a payload gives at most what an emit of its event gives -/
theorem handler_typed {ev h v tv R} (hh : HandlerOk P ev h) (hR : P.effects ev = some R) (hv : v.isValue = true)
    (htv : HasType P Ctx.empty v tv) : ∃ t', HasType P Ctx.empty (h.subst eventLocal v) t' ∧ sub t' (join R .any) = true := by
  obtain ⟨R', th, hR', hth, sth⟩ := hh
  rw [hR] at hR'; cases hR'
  obtain ⟨t', ht', st'⟩ := let_typed hth hv htv (sub_any _)
  exact ⟨t', ht', sub_trans st' (sub_trans sth (join_upper_left _ _))⟩

/-- **Preservation**: a step keeps the store well typed and the expression's type, or makes it smaller -/
theorem preservation (hP : ProgramOk P) {s s' : Expr × Store} (hs : Step P s s') :
    ∀ {t}, HasType P Ctx.empty s.1 t → StoreOk P s.2 →
      (∃ t', HasType P Ctx.empty s'.1 t' ∧ sub t' t = true) ∧ StoreOk P s'.2 := by
  induction hs with
  | @frame F e e' μ μ' _ _ ih =>
    intro t h hμ
    obtain ⟨te, he, k⟩ := frame_typing F h
    obtain ⟨⟨te', he', s'⟩, hμ'⟩ := ih he hμ
    exact ⟨k he' s', hμ'⟩
  | raise => intro t _ hμ; exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | tryStep _ ih =>
    intro t h hμ
    cases h with
    | tryCatch he hh =>
      obtain ⟨⟨_, he', s'⟩, hμ'⟩ := ih he hμ
      exact ⟨⟨_, .tryCatch he' hh, join_mono s' (sub_refl _)⟩, hμ'⟩
  | tryError =>
    intro t h hμ
    cases h with
    | tryCatch _ hh => exact ⟨⟨_, hh, join_upper_right _ _⟩, hμ⟩
  | tryValue =>
    intro t h hμ
    cases h with
    | tryCatch he _ => exact ⟨⟨_, he, join_upper_left _ _⟩, hμ⟩
  | tryFail =>
    intro t h hμ
    cases h with
    | tryCatch _ hh => exact ⟨⟨_, hh, join_upper_right _ _⟩, hμ⟩
  | @readValue x v μ hx =>
    intro t h hμ
    cases h with
    | glob hn =>
      have hc := hμ.1 _ _ _ hn
      simp only at hc
      rw [hx] at hc
      obtain ⟨_, _, tv, htv, st⟩ := hc
      exact ⟨⟨tv, htv, st⟩, hμ⟩
  | readUnset => intro t _ hμ; exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | @readCharged x b μ hx =>
    intro t h hμ
    cases h with
    | glob hn =>
      have hc := hμ.1 _ _ _ hn
      simp only at hc
      rw [hx] at hc
      obtain ⟨_, tb, htb, st⟩ := hc
      exact ⟨⟨tb, htb, st⟩, hμ⟩
  | add va vb =>
    intro t h hμ
    cases h with
    | add ha hb =>
      obtain ⟨ia, _, ha', sa⟩ := items_typed hμ.2.2.2 ha va
      obtain ⟨ib, _, hb', sb⟩ := items_typed hμ.2.2.2 hb vb
      obtain ⟨t', h', s'⟩ := add_typed ha' hb' ia ib
      exact ⟨⟨t', h', sub_trans s' (plus_mono sa sb)⟩, hμ⟩
  | arith va vb =>
    intro t h hμ
    cases h with
    | arith ha hb => exact ⟨arith_typed ha hb va vb, hμ⟩
  | lt => intro t h hμ; cases h; exact ⟨lt_typed, hμ⟩
  | eq => intro t h hμ; cases h; exact ⟨⟨_, .bool, sub_refl _⟩, hμ⟩
  | ite =>
    intro t h hμ
    cases h with
    | ite _ ha hb =>
      refine ⟨?_, hμ⟩
      split
      · exact ⟨_, ha, join_upper_left _ _⟩
      · exact ⟨_, hb, join_upper_right _ _⟩
  | loop =>
    intro t h hμ
    cases h with
    | loop hc hb hd =>
      exact ⟨⟨_, .ite hc (.loop hc hb hb) hd,
        join_least (loop_bound (sub_refl _)) (sub_trans (join_upper_left _ .any) (join_upper_right _ _))⟩, hμ⟩
  | seq => intro t h hμ; cases h with | seq _ hb => exact ⟨⟨_, hb, sub_refl _⟩, hμ⟩
  | @index l i μ vl _ =>
    intro t h hμ
    cases h with
    | index hl _ =>
      refine ⟨?_, hμ⟩
      unfold indexValues
      split
      · cases hl
        obtain ⟨t', h'⟩ := readField_any hμ.2.1 (.ref _ _) _
        exact ⟨t', h', by simp [elementTy, element, Ty.isText]⟩
      obtain ⟨il, _, hl', sl⟩ := items_typed hμ.2.2.2 hl vl
      cases hn : nth (μ.items l) ((asInt i).getD 0) with
      | none => exact ⟨_, .error, sub_never _⟩
      | some v =>
        obtain ⟨tv, hv, sv⟩ := nth_typed _ il hl' hn
        exact ⟨tv, hv, sub_trans sv (elementTy_mono sl)⟩
  | lam => intro t h hμ; cases h with | lam hb => exact ⟨⟨_, .clo hb, sub_refl _⟩, hμ⟩
  | @app y b v μ vv =>
    intro t h hμ
    cases h with
    | app hf ha =>
      cases hf with
      | clo hb =>
        obtain ⟨t', h', s'⟩ := let_typed hb vv ha (sub_any _)
        exact ⟨⟨t', h', by simpa [resultTy] using s'⟩, hμ⟩
  | appOther => intro t _ hμ; exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | @range a b μ _ _ =>
    intro t h hμ
    refine ⟨?_, hμ⟩
    cases h with
    | range =>
      simp only [rangeValues]
      split
      · obtain ⟨e, ht, se⟩ := intList_typed (P := P) (Γ := Ctx.empty) _ _
        exact ⟨_, ht, by simpa [sub] using sub_trans se (int_sub_arithTy _ _)⟩
      · exact ⟨_, .error, sub_never _⟩
  | append va vb =>
    intro t h hμ
    cases h with
    | append ha hb =>
      obtain ⟨ia, _, ha', sa⟩ := items_typed hμ.2.2.2 ha va
      obtain ⟨ib, _, hb', sb⟩ := items_typed hμ.2.2.2 hb vb
      obtain ⟨t', h', s'⟩ := append_typed ha' hb' ia ib
      exact ⟨⟨t', h', sub_trans s' (by simpa using join_mono (listElem_mono sa) (listElem_mono sb))⟩, hμ⟩
  | assign hv =>
    intro t h hμ
    cases h with
    | assign hx he st => exact ⟨⟨_, he, st⟩, hμ.set hx (by simp) hv he st⟩
  | init hv =>
    intro t h hμ
    cases h with
    | init hx hm he st => exact ⟨⟨_, he, st⟩, hμ.set hx hm hv he st⟩
  | letIn hv =>
    intro t h hμ
    cases h with
    | letIn he st hb => exact ⟨let_typed hb hv he st, hμ⟩
  | @call f v fn μ hv hf =>
    intro t h hμ
    cases h with
    | call hf' he st =>
      rw [hf] at hf'; cases hf'
      obtain ⟨⟨tb, hb, sb⟩, _⟩ := hP.1 f fn hf
      exact ⟨⟨tb, .letIn he st hb, sb⟩, hμ⟩
  | broadcastNil => intro t h hμ; cases h; exact ⟨⟨_, .nil, by simp⟩, hμ⟩
  | broadcastCons =>
    intro t h hμ
    cases h with
    | broadcast hf hc hel ha =>
      cases hc with
      | cons hh ht helt =>
        simp only [element, Option.some.injEq] at hel; subst hel
        have h1 := sub_trans (join_upper_left _ _) ha
        have h2 := sub_trans (join_upper_right _ _) ha
        refine ⟨⟨_, .cons (.call hf hh h1) (.broadcast hf ht helt h2) rfl, ?_⟩, hμ⟩
        simp only [sub_list]; exact join_least (sub_refl _) (sub_refl _)
  | @cast v ts μ hv =>
    intro t' h hμ
    cases h with
    | cast _ =>
      refine ⟨?_, hμ⟩
      split
      · rename_i hf
        obtain ⟨t, ht, hfit⟩ := List.any_eq_true.1 hf
        obtain ⟨tv, htv, st⟩ := fits_typed (P := P) (Γ := Ctx.empty) hfit
        exact ⟨tv, htv, sub_trans st (sub_joinAll ht)⟩
      · exact ⟨_, .error, sub_never _⟩
  | @conv v t μ hv =>
    intro t' h hμ
    cases h with
    | conv _ => exact ⟨convertValue_typed μ t, hμ⟩
  | new => intro t h hμ; cases h; exact ⟨⟨_, .ref, sub_refl _⟩, hμ.1, hμ.2.1.alloc _, hμ.2.2⟩
  | @get o f μ vo =>
    intro t h hμ
    cases h with
    | get ho => exact ⟨get_typed hμ.2.1 ho vo, hμ⟩
  | @set o f v μ vo vv =>
    intro t h hμ
    cases h with
    | set ho hw hv st =>
      cases ho <;> simp_all [isValue, Program.writeTy]
      rename_i a p
      simp only [writeField]
      split
      · rename_i hs
        obtain ⟨obj, hobj⟩ := Option.isSome_iff_exists.1 hs
        exact ⟨⟨_, hv, sub_refl _⟩, hμ.1, hμ.2.1.write hobj hw vv hv st, hμ.2.2⟩
      · exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | isA => intro t h hμ; cases h; exact ⟨⟨_, .bool, sub_refl _⟩, hμ⟩
  | failed => intro t h hμ; cases h; exact ⟨⟨_, .bool, sub_refl _⟩, hμ⟩
  | @orElse v b μ hv =>
    intro t h hμ
    cases h with
    | orElse ha hb =>
      refine ⟨?_, hμ⟩
      split
      · exact ⟨_, hb, join_upper_right _ _⟩
      · rename_i full
        have hu : v ≠ Expr.unit := fun hu => by subst hu; simp [Store.items, isEmpty] at full
        exact ⟨_, ha, sub_trans (value_sub_strip ha hv hu) (join_upper_left _ _)⟩
  | handleStep _ ih =>
    intro t h hμ
    cases h with
    | handle hR hh sh hb =>
      obtain ⟨⟨_, hb', s'⟩, hμ'⟩ := ih hb (hμ.push ⟨_, _, hR, hh, sh⟩)
      exact ⟨⟨_, .handle hR hh sh hb', join_mono s' (sub_refl _)⟩, hμ'.restore hμ⟩
  | handleValue => intro t h hμ; cases h with | handle _ _ _ hb => exact ⟨⟨_, hb, join_upper_left _ _⟩, hμ⟩
  | handleError => intro t _ hμ; exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | emitBlock hv hl =>
    intro t h hμ
    cases h with
    | emit hR he =>
      obtain ⟨t', ht', st⟩ := handler_typed (hμ.2.2.1 _ (lookupHandler_mem hl)) hR hv he
      exact ⟨⟨t', .scope ht', st⟩, hμ⟩
  | emitProgram hv _ hp =>
    intro t h hμ
    cases h with
    | emit hR he =>
      obtain ⟨t', ht', st⟩ := handler_typed (hP.2 _ _ hp) hR hv he
      exact ⟨⟨t', .scope ht', st⟩, hμ⟩
  | emitNone => intro t h hμ; cases h; exact ⟨⟨_, .unit, sub_trans (sub_any _) (join_upper_right _ _)⟩, hμ⟩
  | scopeStep _ ih =>
    intro t h hμ
    cases h with
    | scope he =>
      obtain ⟨⟨_, he', s'⟩, hμ'⟩ := ih he (hμ.outer _)
      exact ⟨⟨_, .scope he', s'⟩, hμ'.restore hμ⟩
  | scopeValue => intro t h hμ; cases h with | scope he => exact ⟨⟨_, he, sub_refl _⟩, hμ⟩
  | scopeError => intro t _ hμ; exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | @escape F _ _ _ _ =>
    intro t h hμ
    obtain ⟨_, he, _⟩ := frame_typing F h
    cases he with | abort hv st => exact ⟨⟨_, .abort hv st, sub_never _⟩, hμ⟩
  | forNil =>
    intro t h hμ
    cases h with
    | forIn _ _ _ hd => exact ⟨⟨_, hd, sub_trans (join_upper_left _ _) (join_upper_right _ _)⟩, hμ⟩
  | forOther => intro t _ hμ; exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | @forRef y a t' b v μ _ =>
    intro t h hμ
    cases h with
    | forIn hl sT hb hd =>
      obtain ⟨il, _, hl', sl⟩ := items_typed hμ.2.2.2 hl rfl
      exact ⟨⟨_, .forIn hl' (sub_trans (elementTy_mono sl) sT) hb hd, sub_refl _⟩, hμ⟩
  | @broadcastRef f a t' μ =>
    intro t h hμ
    cases h with
    | broadcast hf he hel hp =>
      obtain ⟨il, _, he', se⟩ := items_typed hμ.2.2.2 he rfl
      obtain ⟨a', hel', sa⟩ := element_mono se hel
      exact ⟨⟨_, .broadcast hf he' hel' (sub_trans sa hp), sub_refl _⟩, hμ⟩
  | @shareNew v t' μ vv _ hfit =>
    intro t h hμ
    cases h with
    | share _ =>
      obtain ⟨tv, hv, sv⟩ := fits_typed (P := P) (Γ := Ctx.empty) hfit
      exact ⟨⟨_, .lref, sub_refl _⟩, hμ.1, hμ.2.1, hμ.2.2.1, hμ.2.2.2.alloc vv hv sv⟩
  | shareBad => intro t _ hμ; exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | push vl vv =>
    intro t h hμ
    cases h with
    | push hl hv => exact push_typed hμ hl vl hv vv
  | setAt _ _ vv =>
    intro t h hμ
    cases h with
    | setAt _ _ hv => exact setAt_typed hμ hv vv
  | @forText y s b v μ _ =>
    intro t h hμ
    cases h with
    | forIn hl sT hb hd =>
      rename_i tl T tb td
      have sT' : sub .codepoint T = true := by cases hl <;> simpa [elementTy, Ty.isText] using sT
      unfold walkText
      split
      · exact ⟨⟨_, hd, sub_trans (join_upper_left _ _) (join_upper_right _ _)⟩, hμ⟩
      · rename_i c rest hp
        obtain ⟨_, h1, s1⟩ := let_typed hb rfl (.codepoint (peel_codepoint hp)) sT'
        exact ⟨⟨_, .forIn (.ofText rest) (by simpa using sT') hb h1, loop_bound s1⟩, hμ⟩
  | @forCons y hd tl b v μ vh vt _ =>
    intro t h hμ
    cases h with
    | forIn hl sT hb hd' =>
      cases hl with
      | cons hh ht he =>
        obtain ⟨_, h1, s1⟩ := let_typed hb vh hh (sub_trans (by simp [elementTy, element]; exact join_upper_left _ _) sT)
        refine ⟨⟨_, .forIn ht (sub_trans ?_ sT) hb h1, loop_bound s1⟩, hμ⟩
        rw [elementTy_of_element he, elementTy_of_element rfl]; exact join_upper_right _ _
  | tryAbort =>
    intro t h hμ
    cases h with | tryCatch he _ => cases he with | abort hv st => exact ⟨⟨_, .abort hv st, sub_never _⟩, hμ⟩
  | scopeAbort =>
    intro t h hμ
    cases h with | scope he => cases he with | abort hv st => exact ⟨⟨_, .abort hv st, sub_never _⟩, hμ⟩
  | handleAbort =>
    intro t h hμ
    cases h with
    | handle _ _ _ hb =>
      cases hb with
      | abort hv st =>
        refine ⟨?_, hμ⟩
        split
        · rename_i hm; obtain ⟨rfl, _⟩ := hm
          exact ⟨_, hv, sub_trans st (join_upper_right _ _)⟩
        · exact ⟨_, .abort hv st, sub_never _⟩

/-- a closed expression is done (a value), failed (an error), aborting (a `break` that found no block of its event,
which the checker's handler rule keeps from happening but the proof does not track), or can step -/
def Progresses (P : Program) (μ : Store) (e : Expr) : Prop :=
  e.isValue = true ∨ (∃ m, e = .error m) ∨ (∃ ev k v, e = .abort ev k v ∧ v.isValue = true) ∨ ∃ s', Step P (e, μ) s'

theorem in_frame {μ : Store} (F : Frame) (hF : F.ready = true) {e : Expr} (h : Progresses P μ e)
    (k : e.isValue = true → Progresses P μ (F.plug e)) : Progresses P μ (F.plug e) := by
  rcases h with hv | ⟨m, rfl⟩ | ⟨ev, k', v, rfl, hv⟩ | ⟨⟨e', μ'⟩, hs⟩
  · exact k hv
  · exact .inr (.inr (.inr ⟨_, .raise hF⟩))
  · exact .inr (.inr (.inr ⟨_, .escape hF hv⟩))
  · exact .inr (.inr (.inr ⟨_, .frame hF hs⟩))

theorem steps {μ : Store} {e : Expr} {s'} (hs : Step P (e, μ) s') : Progresses P μ e := .inr (.inr (.inr ⟨_, hs⟩))

/-- **Progress**: a closed, well-typed expression in a well-typed store is a value, an error, or steps -/
theorem progress {Γ e t} (h : HasType P Γ e t) (hΓ : Γ = Ctx.empty) {μ : Store} (hμ : StoreOk P μ) :
    Progresses P μ e := by
  induction h generalizing μ with
  | bool | int | intIn | num | flt | qty | text | codepoint | unit | nil | fail => exact .inl rfl
  | @cons _ a b _ _ _ _ _ _ ih1 ih2 =>
    exact in_frame (.consL b) rfl (ih1 hΓ hμ) fun va =>
      in_frame (.consR a) va (ih2 hΓ hμ) fun vb => .inl (by simp [Frame.plug, isValue, va, vb])
  | @glob _ x m t hx =>
    have hc := hμ.1 x m t hx
    cases hcell : μ x with
    | none => rw [hcell] at hc; exact hc.elim
    | some c =>
      cases c with
      | unset => exact steps (.readUnset hcell)
      | val v => exact steps (.readValue hcell)
      | charged b => exact steps (.readCharged hcell)
  | loc hy => subst hΓ; simp [Ctx.empty] at hy
  | @add _ a b _ _ _ _ ih1 ih2 =>
    exact in_frame (.addL b) rfl (ih1 hΓ hμ) fun va => in_frame (.addR a) va (ih2 hΓ hμ) fun vb => steps (.add va vb)
  | @arith _ op a b _ _ _ _ ih1 ih2 =>
    exact in_frame (.arithL op b) rfl (ih1 hΓ hμ) fun va => in_frame (.arithR op a) va (ih2 hΓ hμ) fun vb => steps (.arith va vb)
  | @lt _ a b _ _ _ _ ih1 ih2 =>
    exact in_frame (.ltL b) rfl (ih1 hΓ hμ) fun va => in_frame (.ltR a) va (ih2 hΓ hμ) fun vb => steps (.lt va vb)
  | @eq _ s a b _ _ _ _ ih1 ih2 =>
    exact in_frame (.eqL s b) rfl (ih1 hΓ hμ) fun va => in_frame (.eqR s a) va (ih2 hΓ hμ) fun vb => steps (.eq va vb)
  | @ite _ c a b _ _ _ _ _ _ ih0 _ _ => exact in_frame (.ite a b) rfl (ih0 hΓ hμ) fun vc => steps (.ite vc)
  | @loop _ c b _ _ _ _ _ _ _ _ _ ih3 => exact in_frame (.loopLast c b) rfl (ih3 hΓ hμ) fun vd => steps (.loop vd)
  | @seq _ a b _ _ _ _ ih1 _ => exact in_frame (.seq b) rfl (ih1 hΓ hμ) fun va => steps (.seq va)
  | @index _ l i _ _ _ _ ih1 ih2 =>
    exact in_frame (.indexL i) rfl (ih1 hΓ hμ) fun vl => in_frame (.indexR l) vl (ih2 hΓ hμ) fun vi => steps (.index vl vi)
  | lam => exact steps .lam
  | clo => exact .inl rfl
  | @app _ f a _ _ _ _ ih1 ih2 =>
    exact in_frame (.appL a) rfl (ih1 hΓ hμ) fun vf => in_frame (.appR f) vf (ih2 hΓ hμ) fun va => by
      cases hc : isClosure f
      · exact steps (.appOther vf va hc)
      · cases f <;> simp [isClosure] at hc; exact steps (.app va)
  | @range _ a b _ _ _ _ ih1 ih2 =>
    exact in_frame (.rangeL b) rfl (ih1 hΓ hμ) fun va => in_frame (.rangeR a) va (ih2 hΓ hμ) fun vb => steps (.range va vb)
  | @append _ a b _ _ _ _ ih1 ih2 =>
    exact in_frame (.appendL b) rfl (ih1 hΓ hμ) fun va => in_frame (.appendR a) va (ih2 hΓ hμ) fun vb => steps (.append va vb)
  | @assign _ x _ _ _ _ _ _ ih => exact in_frame (.assign x) rfl (ih hΓ hμ) fun v => steps (.assign v)
  | @init _ x _ _ _ _ _ _ _ _ ih => exact in_frame (.init x) rfl (ih hΓ hμ) fun v => steps (.init v)
  | @letIn _ y t _ b _ _ _ _ _ ih _ => exact in_frame (.letIn y t b) rfl (ih hΓ hμ) fun v => steps (.letIn v)
  | @call _ f _ _ _ hf _ _ ih => exact in_frame (.call f) rfl (ih hΓ hμ) fun v => steps (.call v hf)
  | error => exact .inr (.inl ⟨_, rfl⟩)
  | tryCatch _ _ ih _ =>
    rcases ih hΓ hμ with hv | ⟨m, rfl⟩ | ⟨_, _, _, rfl, hv⟩ | ⟨⟨e', μ'⟩, hs⟩
    · cases hf : isFail _
      · exact steps (.tryValue hv hf)
      · obtain ⟨m, rfl⟩ := isFail_eq hf; exact steps .tryFail
    · exact steps .tryError
    · exact steps (.tryAbort hv)
    · exact steps (.tryStep hs)
  | @cast _ _ ts _ _ ih => exact in_frame (.cast ts) rfl (ih hΓ hμ) fun v => steps (.cast v)
  | @conv _ _ t _ _ ih => exact in_frame (.conv t) rfl (ih hΓ hμ) fun v => steps (.conv v)
  | @broadcast _ f _ _ _ _ _ he _ _ ih =>
    refine in_frame (.broadcast f) rfl (ih hΓ hμ) fun v => ?_
    rename_i hel _
    rw [value_list he v hel] at he
    rcases list_value he v with rfl | ⟨hd, tl, rfl, vh, vt⟩ | ⟨a, t, rfl⟩
    · exact steps .broadcastNil
    · exact steps (.broadcastCons vh vt)
    · exact steps .broadcastRef
  | ref => exact .inl rfl
  | lref => exact .inl rfl
  | @share _ e t _ _ ih =>
    refine in_frame (.share t) rfl (ih hΓ hμ) fun v => ?_
    cases hc : (isList e && fits e (.list t))
    · exact steps (.shareBad v hc)
    · simp only [Bool.and_eq_true] at hc; exact steps (.shareNew v hc.1 hc.2)
  | @push _ l v _ _ _ _ ih1 ih2 =>
    exact in_frame (.pushL v) rfl (ih1 hΓ hμ) fun vl => in_frame (.pushR l) vl (ih2 hΓ hμ) fun vv => steps (.push vl vv)
  | @setAt _ l i v _ _ _ _ _ _ ih1 ih2 ih3 =>
    exact in_frame (.setAtL i v) rfl (ih1 hΓ hμ) fun vl => in_frame (.setAtI l v) vl (ih2 hΓ hμ) fun vi =>
      in_frame (.setAtR l i) (by simp [Frame.ready, vl, vi]) (ih3 hΓ hμ) fun vv => steps (.setAt vl vi vv)
  | new => exact steps .new
  | @get _ e f _ _ ih => exact in_frame (.get f) rfl (ih hΓ hμ) fun v => steps (.get v)
  | @set _ e f v _ _ _ _ _ _ _ ih1 ih2 =>
    exact in_frame (.setL f v) rfl (ih1 hΓ hμ) fun vo => in_frame (.setR e f) vo (ih2 hΓ hμ) fun vv => steps (.set vo vv)
  | @isA _ e c _ _ ih => exact in_frame (.isA c) rfl (ih hΓ hμ) fun v => steps (.isA v)
  | @failed _ e _ _ ih => exact in_frame .failed rfl (ih hΓ hμ) fun v => steps (.failed v)
  | @orElse _ a b _ _ _ _ ih _ => exact in_frame (.orElse b) rfl (ih hΓ hμ) fun v => steps (.orElse v)
  | @handle _ ev h b _ _ _ hR hh sh _ _ ih =>
    subst hΓ
    rcases ih rfl (hμ.push ⟨_, _, hR, hh, sh⟩) with hv | ⟨m, rfl⟩ | ⟨_, _, _, rfl, hv⟩ | ⟨⟨e', μ'⟩, hs⟩
    · exact steps (.handleValue hv)
    · exact steps .handleError
    · exact steps (.handleAbort hv)
    · exact steps (.handleStep hs)
  | @emit _ ev e _ _ _ _ ih =>
    refine in_frame (.emit ev) rfl (ih hΓ hμ) fun v => ?_
    cases hl : lookupHandler μ.handlers ev with
    | some p => exact steps (.emitBlock (h := p.1) (k := p.2) v hl)
    | none =>
      cases hp : P.handlers ev with
      | some h => exact steps (.emitProgram v hl hp)
      | none => exact steps (.emitNone v hl hp)
  | @scope _ k e _ _ ih =>
    rcases ih hΓ (hμ.outer k) with hv | ⟨m, rfl⟩ | ⟨_, _, _, rfl, hv⟩ | ⟨⟨e', μ'⟩, hs⟩
    · exact steps (.scopeValue hv)
    · exact steps .scopeError
    · exact steps (.scopeAbort hv)
    · exact steps (.scopeStep hs)
  | @forIn _ y l b d _ _ _ _ _ _ _ _ ih _ ihd =>
    refine in_frame (.forIn y b d) rfl (ih hΓ hμ) fun hv => in_frame (.forInLast y l b) hv (ihd hΓ hμ) fun vd => ?_
    cases l with
    | nil => exact steps (.forNil vd)
    | cons h t => simp only [isValue, Bool.and_eq_true] at hv; exact steps (.forCons hv.1 hv.2 vd)
    | text s => exact steps (.forText vd)
    | lref a t => exact steps (.forRef vd)
    | _ => exact steps (.forOther hv rfl rfl rfl vd)
  | @abort _ ev k e _ _ _ ih => exact in_frame (.abort ev k) rfl (ih hΓ hμ) fun v => .inr (.inr (.inl ⟨ev, k, e, rfl, v⟩))

/-- any number of steps -/
inductive Steps (P : Program) : Expr × Store → Expr × Store → Prop where
  | refl {s} : Steps P s s
  | step {s s' s''} : Step P s s' → Steps P s' s'' → Steps P s s''

/-- **Type safety**: every state a well-typed program reaches is well typed and done, failed, or able to step -/
theorem safety (hP : ProgramOk P) {s s' : Expr × Store} (hs : Steps P s s') :
    ∀ {t}, HasType P Ctx.empty s.1 t → StoreOk P s.2 →
      (∃ t', HasType P Ctx.empty s'.1 t' ∧ sub t' t = true) ∧ StoreOk P s'.2 ∧ Progresses P s'.2 s'.1 := by
  induction hs with
  | refl => intro t h hμ; exact ⟨⟨t, h, sub_refl _⟩, hμ, progress h rfl hμ⟩
  | step h1 _ ih =>
    intro t h hμ
    obtain ⟨⟨t1, h1', s1⟩, hμ1⟩ := preservation hP h1 h hμ
    obtain ⟨⟨t2, h2, s2⟩, hμ2, hp⟩ := ih h1' hμ1
    exact ⟨⟨t2, h2, sub_trans s2 s1⟩, hμ2, hp⟩

end Warp
