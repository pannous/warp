//! Stage 3 outputs and functions with quantities: `print q`, `q in km`, tasks started with quantities, and each function specialised per unit signature
use super::*;

impl Inference {
	/// `print q`, `q in km`, `return q`: the forms of stage 3 written as lists; None for any other list
	pub(super) fn output_form(
		&mut self,
		items: &[Node],
		bracket: &Bracket,
		separator: &Separator,
	) -> Option<Result<(Node, Signature), Stop>> {
		// a one-line block `{ print r.distance }` is its words too
		let call_or_words = matches!(
			(bracket, separator),
			(Bracket::None | Bracket::Curly, Separator::Space) | (Bracket::Round, Separator::None)
		);
		let word = |node: &Node, wanted: &str| matches!(node.drop_meta(), Node::Symbol(w) if w == wanted);
		if call_or_words {
			if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
				if let Some(found) = self.list_word(name, &items[1..]) {
					return Some(found.map(|(arguments, signature)| {
						(
							Node::List(
								[vec![items[0].clone()], arguments].concat(),
								bracket.clone(),
								separator.clone(),
							),
							signature,
						)
					}));
				}
			}
		}
		// `for x in xs` of a list of quantities: x has the element signature
		if let [for_word, variable, in_word, list, body] = items {
			if word(for_word, crate::words::FOR_WORD) && word(in_word, super::super::IN_WORD) {
				if let (Node::Symbol(name), Node::Symbol(list_name)) = (variable.drop_meta(), list.drop_meta()) {
					if let Some(element) = self.lists.get(list_name).cloned() {
						self.variables.insert(name.clone(), element);
						return Some(self.infer(body.clone()).map(|(body, _)| {
							(
								Node::List(
									vec![for_word.clone(), variable.clone(), in_word.clone(), list.clone(), body],
									bracket.clone(),
									separator.clone(),
								),
								vec![],
							)
						}));
					}
				}
			}
		}
		match items {
			// `str(q)`, `print q`: the quantity as its text in the units it is written in, a whole list or object of them too
			[head, value]
				if call_or_words && (word(head, PRINT_WORD) || TEXT_WORDS.iter().any(|text| word(head, text))) =>
			{
				let printed = word(head, PRINT_WORD);
				let amount_text = if printed { PRINTED_AMOUNT_TEXT } else { AMOUNT_TEXT };
				Some((|| {
					let text = match self.whole_text(value, amount_text) {
						Some(text) => text?,
						None => {
							let (written, conversion) = (value.clone(), conversion_target(value));
							let (value, signature) = self.infer(value.clone())?;
							if signature.is_empty() {
								let call = Node::List(vec![head.clone(), value], bracket.clone(), separator.clone());
								return Ok((call, vec![]));
							}
							let shown_units = conversion.unwrap_or_else(|| self.shown_factors(&written, &signature));
							self.as_text(value, &signature, Some(shown_units), amount_text)?
						}
					};
					match printed {
						true => Ok((Node::List(vec![head.clone(), text], bracket.clone(), separator.clone()), vec![])),
						false => Ok((text, vec![])),
					}
				})())
			}
			[head, value] if call_or_words && word(head, SI_AMOUNT_WORD) => {
				Some(self.infer(value.clone()).map(|(value, _)| (value, vec![])))
			}
			[head, value] if *bracket == Bracket::None && word(head, RETURN_WORD) => Some((|| {
				let (value, signature) = self.infer(value.clone())?;
				if let Some(returns) = self.returns.last_mut() {
					returns.push(signature.clone());
				}
				Ok((Node::List(vec![head.clone(), value], bracket.clone(), separator.clone()), signature))
			})()),
			[quantity, in_word, unit]
				if *bracket == Bracket::None
					&& word(in_word, super::super::IN_WORD)
					&& super::super::unit_expression(unit).is_some() =>
			{
				Some(self.converted(quantity.clone(), unit))
			}
			_ => None,
		}
	}

	/// `task·go(f, args)` of a function with quantities starts its specialisation (the job holds its result in SI amounts)
	pub(super) fn task_start(
		&mut self,
		items: &[Node],
		bracket: &Bracket,
		separator: &Separator,
	) -> Option<Result<(Node, Signature), Stop>> {
		let (head, call) = items.split_first()?;
		if declarations::word(head) != declarations::TASK_GO {
			return None;
		}
		let started = match self.specialised_call(call)? {
			Ok((Node::List(started, _, _), _)) => started,
			Ok(_) => return None,
			Err(stop) => return Some(Err(stop)),
		};
		self.started.insert(declarations::word(&call[0]), declarations::word(&started[0]));
		Some(Ok((Node::List([vec![head.clone()], started].concat(), bracket.clone(), separator.clone()), vec![])))
	}

	/// The reads and controls of a task (`await job`: `task·value(job, f)`) name the specialisation it started
	pub(super) fn with_started_names(&self, items: Vec<Node>) -> Vec<Node> {
		if !items.first().is_some_and(|head| declarations::is_task_marker(&declarations::word(head))) {
			return items;
		}
		items
			.into_iter()
			.map(|part| match self.started.get(&declarations::word(&part)) {
				Some(specialised) => Node::Symbol(specialised.clone()),
				None => part,
			})
			.collect()
	}

	/// `f(a, b)` of a user function with quantities (or units in its body): the call of its specialisation
	pub(super) fn specialised_call(&mut self, items: &[Node]) -> Option<Result<(Node, Signature), Stop>> {
		let (head, arguments) = items.split_first()?;
		let Node::Symbol(name) = head.drop_meta() else {
			return None;
		};
		let definition = self.definitions.get(&(name.clone(), arguments.len()))?.clone();
		let defaults = definition.defaults[arguments.len()..].iter().flatten().cloned();
		let mut lowered = vec![];
		let mut signatures = vec![];
		for argument in arguments.iter().cloned().chain(defaults) {
			match self.infer(argument) {
				Ok((argument, signature)) => {
					lowered.push(argument);
					signatures.push(signature);
				}
				Err(stop) => return Some(Err(stop)),
			}
		}
		if signatures.iter().all(Vec::is_empty) && !self.has_units(&definition) {
			return None;
		}
		Some(self.specialise(name, &definition, signatures).map(|(specialised, result)| {
			(Node::List([vec![Node::Symbol(specialised)], lowered].concat(), Bracket::Round, Separator::None), result)
		}))
	}

	/// The specialisation of a function for the unit signatures of its arguments: its name and result signature
	pub(super) fn specialise(
		&mut self,
		name: &str,
		definition: &Definition,
		signatures: Vec<Signature>,
	) -> Result<(String, Signature), Stop> {
		let key = (name.to_string(), signatures.clone());
		if let Some(done) = self.specialised.get(&key) {
			return Ok(done.clone());
		}
		if let Some(progressing) = self.in_progress.iter_mut().find(|progressing| progressing.key == key) {
			progressing.recursive = true;
			return progressing.result.clone().map(|result| (progressing.name.clone(), result)).ok_or(Stop::Recursive);
		}
		if self
			.in_progress
			.iter()
			.any(|progressing| progressing.key.0 == name && progressing.key.1.len() == signatures.len())
		{
			return Err(Stop::Unsupported); // recursion that changes the arguments' units
		}
		let specialised = format!("{name}{SPECIALISATION_SEPARATOR}{}", self.numbered);
		self.numbered += 1;
		self.in_progress.push(InProgress {
			key: key.clone(),
			name: specialised.clone(),
			recursive: false,
			result: None,
		});
		let mut inferred = self.infer_body(name, definition, &signatures);
		// a recursive function: its recursive calls give the result of the first pass, which the second pass must confirm
		if let (Ok((_, result)), Some(progressing)) =
			(&inferred, self.in_progress.last_mut().filter(|progressing| progressing.recursive))
		{
			progressing.result = Some(result.clone());
			let provisional = result.clone();
			inferred = self.infer_body(name, definition, &signatures);
			if let Ok((_, result)) = &inferred {
				if *result != provisional {
					inferred = Err(Stop::Error(format!(
						"DimensionError: {name} returns {} and {}",
						shown(&provisional),
						shown(result)
					)));
				}
			}
		}
		self.in_progress.pop();
		let (body, result) = inferred?;
		let head = Node::List(
			std::iter::once(Node::Symbol(specialised.clone())).chain(definition.declared.iter().cloned()).collect(),
			Bracket::Round,
			Separator::None,
		);
		self.new_definitions.push(Node::Key(Box::new(head), Op::Define, Box::new(body)));
		self.specialised.insert(key, (specialised.clone(), result.clone()));
		Ok((specialised, result))
	}

	/// A function with unit literals in its body or defaults, or reading a global that holds a quantity
	pub(super) fn has_units(&self, definition: &Definition) -> bool {
		let body = &definition.body;
		let holds_quantity = |name: &String| {
			self.globals.get(name).or_else(|| self.variables.get(name)).is_some_and(|signature| !signature.is_empty())
		};
		mentions_units(body)
			|| definition.defaults.iter().flatten().any(mentions_units)
			|| body.declared_globals::<Vec<String>>().iter().any(holds_quantity)
			|| self
				.globals
				.iter()
				.any(|(name, signature)| !signature.is_empty() && super::super::mentions_word(body, name))
	}

	/// The body of a function lowered for the unit signatures of its parameters, and its result signature
	pub(super) fn infer_body(
		&mut self,
		name: &str,
		definition: &Definition,
		signatures: &[Signature],
	) -> Result<(Node, Signature), Stop> {
		// the globals it writes keep their signatures: main's `global` ones and those it declares `global x` itself
		let mut scope = self.globals.clone();
		for name in definition.body.declared_globals::<Vec<String>>() {
			if let Some(signature) = self.variables.get(&name) {
				scope.insert(name, signature.clone());
			}
		}
		scope.extend(definition.parameters.iter().cloned().zip(signatures.iter().cloned()));
		let outer = std::mem::replace(&mut self.variables, scope);
		self.returns.push(vec![]);
		let inferred = self.infer(definition.body.clone());
		let returns = self.returns.pop().unwrap_or_default();
		self.variables = outer;
		let (body, result) = inferred?;
		// every `return` gives the result's units: the first one decides, the others and the last statement agree
		let result = returns.first().cloned().unwrap_or(result);
		if let Some(other) = returns.iter().find(|signature| **signature != result) {
			return Err(Stop::Error(format!("DimensionError: {name} returns {} and {}", shown(&result), shown(other))));
		}
		Ok((body, result))
	}

	pub(super) fn infer_list(
		&mut self,
		items: Vec<Node>,
		bracket: Bracket,
		separator: Separator,
	) -> Result<(Node, Signature), Stop> {
		if let Some(output) = self.output_form(&items, &bracket, &separator) {
			return output;
		}
		if let Some(task) = self.task_start(&items, &bracket, &separator) {
			return task;
		}
		let items = self.with_started_names(items);
		// `(q as km)`: parentheses around one expression keep its signature
		if let ([inner], Bracket::Round) = (items.as_slice(), &bracket) {
			if matches!(inner.drop_meta(), Node::Key(..)) || conversion_target(inner).is_some() {
				let (inner, signature) = self.infer(inner.clone())?;
				return Ok((Node::List(vec![inner], bracket, separator), signature));
			}
		}
		// a written object `{dist:500m name:"run"}` (only `:` entries: `{x = 1 m}` is a block); as the program's value its
		// fields' units go into `warp.units`
		let list = Node::List(items.clone(), bracket.clone(), separator.clone());
		let is_written_object =
			object_entries(&list).is_some_and(|entries| entries.iter().all(|(_, op, _)| *op == Op::Colon));
		if let Some(object) = self.object_literal(&list).filter(|_| is_written_object) {
			let (object, fields) = object?;
			if self.at_top {
				self.result_fields = Some(fields.into_iter().filter(|(_, signature)| !signature.is_empty()).collect());
			}
			return Ok((object, vec![]));
		}
		if bracket == Bracket::Round {
			if let Some(call) = self.constructed(&items) {
				return call;
			}
		}
		if bracket == Bracket::Round && separator == Separator::None {
			if let Some(call) = self.specialised_call(&items) {
				return call;
			}
		}
		let statements = matches!(separator, Separator::Semicolon | Separator::Newline) || bracket == Bracket::Curly;
		let is_loop = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if LOOP_WORDS.contains(&word.as_str()));
		if is_loop {
			self.loop_instance(&items);
		}
		// `5 km`: an amount and a unit
		// the amount is a value: a number, an expression or a variable, never a word like `use` in `use m` (the libm module)
		let is_amount = |item: &Node| match item.drop_meta() {
			Node::Symbol(name) => self.variables.contains_key(name),
			Node::Text(_) | Node::Char(_) => false,
			_ => true,
		};
		if let ([amount, unit], Bracket::None, Separator::Space) = (items.as_slice(), &bracket, &separator) {
			if let Some(unit) = self.unit_word(unit).filter(|_| is_amount(amount)) {
				let outer = std::mem::replace(&mut self.at_top, false);
				let inferred = self.amount_in(amount.clone(), unit);
				self.at_top = outer;
				return inferred;
			}
		}
		let is_host_call = bracket == Bracket::Round
			&& matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == HOST_CALL);
		// `(r.distance)`, `(a + b)`: parentheses around one expression group it (a lone word `(f)` may be a call)
		let grouping = bracket == Bracket::Round
			&& matches!(items.as_slice(), [single] if !matches!(single.drop_meta(), Node::Symbol(_)));
		let in_host_call = self.in_host_call || is_host_call;
		let outer_host_call = std::mem::replace(&mut self.in_host_call, in_host_call);
		let mut lowered = vec![];
		let mut signatures = vec![];
		let count = items.len();
		for (index, item) in items.into_iter().enumerate() {
			// the program's last statement may show a whole list of quantities
			if statements && index + 1 == count && self.at_top {
				// a final object: its fields' units go into the `units` entry
				let fields = self.objects.get(&item.name()).cloned().or_else(|| self.list_instance_fields(&item));
				if let Some(fields) = fields.filter(|_| matches!(item.drop_meta(), Node::Symbol(_))) {
					self.result_fields =
						Some(fields.iter().filter(|(_, signature)| !signature.is_empty()).cloned().collect());
					lowered.push(item);
					signatures.push(vec![]);
					continue;
				}
				if let Some(text) = self.final_list(&item) {
					lowered.push(text?);
					signatures.push(vec![]);
					continue;
				}
			}
			let outer = std::mem::replace(&mut self.at_top, false);
			let inferred = self.infer(item);
			self.at_top = outer;
			let (item, signature) = inferred?;
			lowered.push(item);
			signatures.push(signature);
		}
		let in_host_call = std::mem::replace(&mut self.in_host_call, outer_host_call);
		let signature = if statements || grouping {
			signatures.last().cloned().unwrap_or_default()
		} else if is_loop || in_host_call || signatures.iter().all(Vec::is_empty) {
			vec![]
		} else {
			return Err(Stop::Unsupported); // a call, print, a list literal or interpolation of a quantity
		};
		Ok((Node::List(lowered, bracket, separator), signature))
	}
}
