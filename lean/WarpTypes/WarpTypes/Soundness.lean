import WarpTypes.Lemmas

/-! Progress and preservation for W0, and type safety: a well-typed program never gets stuck. -/

namespace Warp
open Ty Expr

variable {P : Program}

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
  case addL =>
    cases h with | add ha hb sa sb =>
    exact ⟨_, ha, fun h' s => ⟨_, .add h' hb (addable_mono s sa) sb, plus_mono s (sub_refl _) sa sb⟩⟩
  case addR =>
    cases h with | add ha hb sa sb =>
    exact ⟨_, hb, fun h' s => ⟨_, .add ha h' sa (addable_mono s sb), plus_mono (sub_refl _) s sa sb⟩⟩
  case ltL =>
    cases h with | lt ha hb sa sb =>
    exact ⟨_, ha, fun h' s => ⟨_, .lt h' hb (sub_trans s sa) sb, sub_refl _⟩⟩
  case ltR =>
    cases h with | lt ha hb sa sb =>
    exact ⟨_, hb, fun h' s => ⟨_, .lt ha h' sa (sub_trans s sb), sub_refl _⟩⟩
  case eqL => cases h with | eq ha hb => exact ⟨_, ha, fun h' _ => ⟨_, .eq h' hb, sub_refl _⟩⟩
  case eqR => cases h with | eq ha hb => exact ⟨_, hb, fun h' _ => ⟨_, .eq ha h', sub_refl _⟩⟩
  case ite => cases h with | ite hc ha hb => exact ⟨_, hc, fun h' _ => ⟨_, .ite h' ha hb, sub_refl _⟩⟩
  case seq => cases h with | seq ha hb => exact ⟨_, ha, fun h' _ => ⟨_, .seq h' hb, sub_refl _⟩⟩
  case indexL =>
    cases h with | index hl he hi si =>
    refine ⟨_, hl, fun h' s => ?_⟩
    obtain ⟨e', he', se⟩ := element_mono s he
    exact ⟨_, .index h' he' hi si, se⟩
  case indexR =>
    cases h with | index hl he hi si =>
    exact ⟨_, hi, fun h' s => ⟨_, .index hl he h' (sub_trans s si), sub_refl _⟩⟩
  case appendL =>
    cases h with | append ha hea hb heb =>
    refine ⟨_, ha, fun h' s => ?_⟩
    obtain ⟨e', he', se⟩ := element_mono s hea
    exact ⟨_, .append h' he' hb heb, by simpa using join_mono se (sub_refl _)⟩
  case appendR =>
    cases h with | append ha hea hb heb =>
    refine ⟨_, hb, fun h' s => ?_⟩
    obtain ⟨e', he', se⟩ := element_mono s heb
    exact ⟨_, .append ha hea h' he', by simpa using join_mono (sub_refl _) se⟩
  case assign =>
    cases h with | assign hx he st => exact ⟨_, he, fun h' s => ⟨_, .assign hx h' (sub_trans s st), sub_refl _⟩⟩
  case init =>
    cases h with | init hx hm he st => exact ⟨_, he, fun h' s => ⟨_, .init hx hm h' (sub_trans s st), sub_refl _⟩⟩
  case letIn =>
    cases h with | letIn he st hb => exact ⟨_, he, fun h' s => ⟨_, .letIn h' (sub_trans s st) hb, sub_refl _⟩⟩
  case call =>
    cases h with | call hf he st => exact ⟨_, he, fun h' s => ⟨_, .call hf h' (sub_trans s st), sub_refl _⟩⟩
  case cast => cases h with | cast he => exact ⟨_, he, fun h' _ => ⟨_, .cast h', sub_refl _⟩⟩
  case broadcast =>
    cases h with | broadcast hf he hel ha =>
    refine ⟨_, he, fun h' s => ?_⟩
    obtain ⟨a', he', sa⟩ := element_mono s hel
    exact ⟨_, .broadcast hf h' he' (sub_trans sa ha), sub_refl _⟩
  case get =>
    cases h with | get he hr =>
    refine ⟨_, he, fun h' s => ?_⟩
    obtain ⟨t', hr', st⟩ := readTy_mono s hr
    exact ⟨_, .get h' hr', st⟩
  case setL =>
    cases h with | set he hw hv st =>
    refine ⟨_, he, fun h' s => ?_⟩
    obtain ⟨t', hw', st'⟩ := writeTy_anti s hw
    exact ⟨_, .set h' hw' hv (sub_trans st st'), sub_refl _⟩
  case setR =>
    cases h with | set he hw hv st =>
    exact ⟨_, hv, fun h' s => ⟨_, .set he hw h' (sub_trans s st), s⟩⟩
  case isA => cases h with | isA he => exact ⟨_, he, fun h' _ => ⟨_, .isA h', sub_refl _⟩⟩

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

theorem StoreOk.set {μ : Store} (hμ : StoreOk P μ) {x m t v tv} (hx : P.names x = some (m, t)) (hm : m ≠ .charged)
    (hv : v.isValue = true) (htv : HasType P Ctx.empty v tv) (st : sub tv t = true) :
    StoreOk P (μ.set x (.val v)) := by
  refine ⟨fun z mz tz hz => ?_, hμ.2⟩
  show CellOk P mz tz (if z = x then _ else _)
  split
  · subst_vars; rw [hx] at hz; cases hz; exact ⟨hm, hv, tv, htv, st⟩
  · exact hμ.1 z mz tz hz

/-- **Preservation**: a step keeps the store well typed and the expression's type, or makes it smaller -/
theorem preservation (hP : FunsOk P) {s s' : Expr × Store} (hs : Step P s s') :
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
    | add ha hb sa sb => exact ⟨add_typed ha hb va vb sa sb, hμ⟩
  | lt => intro t h hμ; cases h; exact ⟨⟨_, .bool, sub_refl _⟩, hμ⟩
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
    | loop hc hb => exact ⟨⟨_, .ite hc (.seq hb (.loop hc hb)) .unit, by simp [join, sub_refl]⟩, hμ⟩
  | seq => intro t h hμ; cases h with | seq _ hb => exact ⟨⟨_, hb, sub_refl _⟩, hμ⟩
  | @index l i μ vl _ =>
    intro t h hμ
    cases h with
    | index hl he _ _ =>
      refine ⟨?_, hμ⟩
      cases hn : nth l ((asInt i).getD 0) with
      | none => exact ⟨_, .error, sub_never _⟩
      | some v => simpa using nth_typed _ vl hl he hn
  | append va vb =>
    intro t h hμ
    cases h with
    | append ha hea hb heb => exact ⟨append_typed hb vb heb va ha hea, hμ⟩
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
      obtain ⟨⟨tb, hb, sb⟩, _⟩ := hP f fn hf
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
  | @cast v t μ hv =>
    intro t' h hμ
    cases h with
    | cast _ =>
      refine ⟨?_, hμ⟩
      split
      · rename_i hf; obtain ⟨tv, htv, st⟩ := fits_typed (P := P) (Γ := Ctx.empty) hf; exact ⟨tv, htv, st⟩
      · exact ⟨_, .error, sub_never _⟩
  | new => intro t h hμ; cases h; exact ⟨⟨_, .ref, sub_refl _⟩, hμ.1, hμ.2.alloc _⟩
  | @get o f μ vo =>
    intro t h hμ
    cases h with
    | get ho hr =>
      refine ⟨?_, hμ⟩
      cases ho <;> simp_all [isValue, Program.readTy]
      exact readField_typed hμ.2 .ref rfl hr
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
        exact ⟨⟨_, hv, sub_refl _⟩, hμ.1, hμ.2.write hobj hw vv hv st⟩
      · exact ⟨⟨_, .error, sub_never _⟩, hμ⟩
  | isA => intro t h hμ; cases h; exact ⟨⟨_, .bool, sub_refl _⟩, hμ⟩

/-- a closed expression is done (a value), failed (an error), or can step -/
def Progresses (P : Program) (μ : Store) (e : Expr) : Prop :=
  e.isValue = true ∨ (∃ m, e = .error m) ∨ ∃ s', Step P (e, μ) s'

theorem in_frame {μ : Store} (F : Frame) (hF : F.ready = true) {e : Expr} (h : Progresses P μ e)
    (k : e.isValue = true → Progresses P μ (F.plug e)) : Progresses P μ (F.plug e) := by
  rcases h with hv | ⟨m, rfl⟩ | ⟨⟨e', μ'⟩, hs⟩
  · exact k hv
  · exact .inr (.inr ⟨_, .raise hF⟩)
  · exact .inr (.inr ⟨_, .frame hF hs⟩)

theorem steps {μ : Store} {e : Expr} {s'} (hs : Step P (e, μ) s') : Progresses P μ e := .inr (.inr ⟨_, hs⟩)

/-- **Progress**: a closed, well-typed expression in a well-typed store is a value, an error, or steps -/
theorem progress {Γ e t} (h : HasType P Γ e t) (hΓ : Γ = Ctx.empty) {μ : Store} (hμ : StoreOk P μ) :
    Progresses P μ e := by
  induction h with
  | bool | int | num | text | unit | nil => exact .inl rfl
  | @cons _ a b _ _ _ _ _ _ ih1 ih2 =>
    exact in_frame (.consL b) rfl (ih1 hΓ) fun va =>
      in_frame (.consR a) va (ih2 hΓ) fun vb => .inl (by simp [Frame.plug, isValue, va, vb])
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
  | @add _ a b _ _ _ _ _ _ ih1 ih2 =>
    exact in_frame (.addL b) rfl (ih1 hΓ) fun va => in_frame (.addR a) va (ih2 hΓ) fun vb => steps (.add va vb)
  | @lt _ a b _ _ _ _ _ _ ih1 ih2 =>
    exact in_frame (.ltL b) rfl (ih1 hΓ) fun va => in_frame (.ltR a) va (ih2 hΓ) fun vb => steps (.lt va vb)
  | @eq _ a b _ _ _ _ ih1 ih2 =>
    exact in_frame (.eqL b) rfl (ih1 hΓ) fun va => in_frame (.eqR a) va (ih2 hΓ) fun vb => steps (.eq va vb)
  | @ite _ c a b _ _ _ _ _ _ ih0 _ _ => exact in_frame (.ite a b) rfl (ih0 hΓ) fun vc => steps (.ite vc)
  | loop => exact steps .loop
  | @seq _ a b _ _ _ _ ih1 _ => exact in_frame (.seq b) rfl (ih1 hΓ) fun va => steps (.seq va)
  | @index _ l i _ _ _ _ _ _ _ ih1 ih2 =>
    exact in_frame (.indexL i) rfl (ih1 hΓ) fun vl => in_frame (.indexR l) vl (ih2 hΓ) fun vi => steps (.index vl vi)
  | @append _ a b _ _ _ _ _ _ _ _ ih1 ih2 =>
    exact in_frame (.appendL b) rfl (ih1 hΓ) fun va => in_frame (.appendR a) va (ih2 hΓ) fun vb => steps (.append va vb)
  | @assign _ x _ _ _ _ _ _ ih => exact in_frame (.assign x) rfl (ih hΓ) fun v => steps (.assign v)
  | @init _ x _ _ _ _ _ _ _ _ ih => exact in_frame (.init x) rfl (ih hΓ) fun v => steps (.init v)
  | @letIn _ y t _ b _ _ _ _ _ ih _ => exact in_frame (.letIn y t b) rfl (ih hΓ) fun v => steps (.letIn v)
  | @call _ f _ _ _ hf _ _ ih => exact in_frame (.call f) rfl (ih hΓ) fun v => steps (.call v hf)
  | error => exact .inr (.inl ⟨_, rfl⟩)
  | tryCatch _ _ ih _ =>
    rcases ih hΓ with hv | ⟨m, rfl⟩ | ⟨⟨e', μ'⟩, hs⟩
    · exact steps (.tryValue hv)
    · exact steps .tryError
    · exact steps (.tryStep hs)
  | @cast _ _ t _ _ ih => exact in_frame (.cast t) rfl (ih hΓ) fun v => steps (.cast v)
  | @broadcast _ f _ _ _ _ _ he _ _ ih =>
    refine in_frame (.broadcast f) rfl (ih hΓ) fun v => ?_
    rename_i hel _
    rw [value_list he v hel] at he
    rcases list_value he v with rfl | ⟨hd, tl, rfl, vh, vt⟩
    · exact steps .broadcastNil
    · exact steps (.broadcastCons vh vt)
  | ref => exact .inl rfl
  | new => exact steps .new
  | @get _ e f _ _ _ _ ih => exact in_frame (.get f) rfl (ih hΓ) fun v => steps (.get v)
  | @set _ e f v _ _ _ _ _ _ _ ih1 ih2 =>
    exact in_frame (.setL f v) rfl (ih1 hΓ) fun vo => in_frame (.setR e f) vo (ih2 hΓ) fun vv => steps (.set vo vv)
  | @isA _ e c _ _ ih => exact in_frame (.isA c) rfl (ih hΓ) fun v => steps (.isA v)

/-- any number of steps -/
inductive Steps (P : Program) : Expr × Store → Expr × Store → Prop where
  | refl {s} : Steps P s s
  | step {s s' s''} : Step P s s' → Steps P s' s'' → Steps P s s''

/-- **Type safety**: every state a well-typed program reaches is well typed and done, failed, or able to step -/
theorem safety (hP : FunsOk P) {s s' : Expr × Store} (hs : Steps P s s') :
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
