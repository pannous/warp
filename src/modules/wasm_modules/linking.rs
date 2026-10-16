//! Linking a module's exports at run time: each import calls the module instantiated once per run, its C values copied across
use super::*;

/// kind, data, value (wasm_emitter/type_manager.rs $Node), each mutable
#[cfg(feature = "native")]
pub(crate) const NODE_FIELDS: usize = 3;

/// Link the program's imports from WebAssembly modules: each is a host function that calls the export of the module,
/// instantiated in the run's store at its first call
#[cfg(feature = "native")]
pub fn link(linker: &mut wasmtime::Linker<crate::host::HostState>, module: &wasmtime::Module) -> wasmtime::Result<()> {
	for import in module.imports().filter(|import| is_module_path(import.module())) {
		let wasmtime::ExternType::Func(function_type) = import.ty() else {
			continue;
		};
		let (path, name) = (import.module().to_string(), import.name().to_string());
		let role = exports(&path).get(&name).map_or(Role::Function, |export| export.role);
		linker.func_new(import.module(), import.name(), function_type, move |mut caller, arguments, results| {
			let instance = instance_of(&mut caller, &path)?;
			let global_name = name.strip_prefix(SETTER_PREFIX).unwrap_or(&name);
			let global = |caller: &mut wasmtime::Caller<'_, crate::host::HostState>| {
				instance
					.get_global(caller, global_name)
					.ok_or_else(|| wasmtime::format_err!("{path} exports no global {global_name}"))
			};
			match role {
				Role::Global { .. } => results[0] = global(&mut caller)?.get(&mut caller),
				Role::Setter => {
					global(&mut caller)?.set(&mut caller, arguments[0])?;
					results[0] = arguments[0];
				}
				Role::Function => {
					let function = instance
						.get_func(&mut caller, &name)
						.ok_or_else(|| wasmtime::format_err!("{path} exports no function {name}"))?;
					match exports(&path).get(&name) {
						Some(export) if export.crosses_c_values() => {
							call_c(&mut caller, instance, function, export, arguments, results)?
						}
						Some(export) if export.node_result.is_some() => {
							call_with_values(&mut caller, instance, function, arguments, results)?
						}
						_ => function.call(&mut caller, arguments, results)?,
					}
				}
			}
			Ok(())
		})?;
	}
	Ok(())
}

/// A call of a C function of a module whose header says how its values cross (CParameter, CResult): each text argument
/// (a NUL-terminated copy in the program's memory, or as many bytes as its length says) is copied into a block of the
/// module's `malloc`, each out-pointer gets a NULL slot there, each buffer a block of its capacity and a length slot; the
/// result is read back (a text, an unsigned number, what the first out-pointer or buffer received) before the blocks
/// are freed
#[cfg(feature = "native")]
pub(crate) fn call_c(
	caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
	instance: wasmtime::Instance,
	function: wasmtime::Func,
	export: &Export,
	arguments: &[wasmtime::Val],
	results: &mut [wasmtime::Val],
) -> wasmtime::Result<()> {
	let name = export.signature.name;
	let program_memory = crate::host::exported_memory(caller)
		.ok_or_else(|| wasmtime::format_err!("{name}: the program has no memory for its text"))?;
	let memory = instance
		.get_memory(&mut *caller, "memory")
		.ok_or_else(|| wasmtime::format_err!("{name}: its module exports no memory"))?;
	let mut blocks = ModuleBlocks { name, instance, memory, allocated: vec![], out_slots: vec![], buffers: vec![] };
	let module_arguments = blocks.passed(caller, program_memory, &export.c_parameters, arguments)?;
	let mut returned = [wasmtime::Val::I32(0)];
	let result_count = function.ty(&*caller).results().len();
	function.call(&mut *caller, &module_arguments, &mut returned[..result_count])?;
	let result = blocks.received(caller, &export.c_result, returned[0])?;
	if let Some(slot) = results.first_mut() {
		*slot = result;
	}
	blocks.freed(caller)
}

/// The blocks a C call allocates in its module's memory with `malloc`, freed after the call
#[cfg(feature = "native")]
struct ModuleBlocks {
	name: &'static str,
	instance: wasmtime::Instance,
	memory: wasmtime::Memory,
	allocated: Vec<i32>,
	out_slots: Vec<i32>,
	/// each buffer and the slot of its length
	buffers: Vec<(i32, i32)>,
}

#[cfg(feature = "native")]
impl ModuleBlocks {
	fn allocated(
		&mut self,
		caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
		bytes: &[u8],
	) -> wasmtime::Result<i32> {
		let name = self.name;
		let malloc = self.instance.get_typed_func::<i32, i32>(&mut *caller, "malloc").map_err(|_| {
			wasmtime::format_err!("{name}: its module exports no malloc(size) to pass a text or out-pointer")
		})?;
		let block = malloc.call(&mut *caller, bytes.len() as i32)?;
		self.memory.write(&mut *caller, block as usize, bytes)?;
		self.allocated.push(block);
		Ok(block)
	}

	/// The arguments the C function takes for the program's: texts copied into blocks, slots for out-pointers and buffers
	fn passed(
		&mut self,
		caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
		program_memory: wasmtime::Memory,
		parameters: &[CParameter],
		arguments: &[wasmtime::Val],
	) -> wasmtime::Result<Vec<wasmtime::Val>> {
		use wasmtime::Val;
		let name = self.name;
		let too_few = || wasmtime::format_err!("{name}: too few arguments");
		let mut program_arguments = arguments.iter().peekable();
		let mut module_arguments = Vec::new();
		let (mut capacity, mut buffer) = (0, 0);
		for (index, parameter) in parameters.iter().enumerate() {
			let argument = match parameter {
				CParameter::Number | CParameter::TextLength => *program_arguments.next().ok_or_else(too_few)?,
				CParameter::Text => {
					let address = program_arguments.next().ok_or_else(too_few)?.unwrap_i32() as usize;
					let memory = program_memory.data(&*caller);
					let mut text = match parameters.get(index + 1) {
						Some(CParameter::TextLength) => memory
							.get(address..address + count_of(program_arguments.peek().ok_or_else(too_few)?))
							.unwrap_or_default(),
						_ => c_text(memory, address),
					}
					.to_vec();
					text.push(0);
					Val::I32(self.allocated(caller, &text)?)
				}
				CParameter::Out => {
					let slot = self.allocated(caller, &[0; 4])?;
					self.out_slots.push(slot);
					Val::I32(slot)
				}
				CParameter::Buffer => {
					capacity = count_of(program_arguments.next().ok_or_else(too_few)?);
					buffer = self.allocated(caller, &vec![0; capacity])?;
					Val::I32(buffer)
				}
				CParameter::BufferLength => {
					let slot = self.allocated(caller, &(capacity as u32).to_le_bytes())?;
					self.buffers.push((buffer, slot));
					Val::I32(slot)
				}
			};
			module_arguments.push(argument);
		}
		Ok(module_arguments)
	}

	/// The result as the header says it crosses: a number, an unsigned one, a text, or what the first out-pointer or
	/// buffer received
	fn received(
		&self,
		caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
		result: &CResult,
		returned: wasmtime::Val,
	) -> wasmtime::Result<wasmtime::Val> {
		use wasmtime::Val;
		let name = self.name;
		match result {
			CResult::Number => Ok(returned),
			CResult::Unsigned => Ok(Val::I64(returned.unwrap_i32() as u32 as i64)),
			CResult::Text => {
				let text = self.text_at(caller, returned.unwrap_i32() as u32);
				self.text_node(caller, text.as_deref())
			}
			CResult::Out { text } => {
				let received = self.u32_at(caller, self.out_slots[0] as usize);
				if received == 0 {
					return Err(wasmtime::format_err!(
						"{name} gave nothing through its out-pointer (C status {})",
						returned.i32().unwrap_or(0)
					));
				}
				match text {
					true => {
						let text = self.text_at(caller, received);
						self.text_node(caller, text.as_deref())
					}
					false => Ok(Val::I32(received as i32)),
				}
			}
			CResult::Buffer => {
				let status = returned.i32().unwrap_or(0);
				if status != 0 {
					return Err(wasmtime::format_err!("{name} failed (C status {status})"));
				}
				let (block, length_slot) = self.buffers[0];
				let length = self.u32_at(caller, length_slot as usize) as usize;
				let bytes = self
					.memory
					.data(&*caller)
					.get(block as usize..block as usize + length)
					.unwrap_or_default()
					.to_vec();
				self.text_node(caller, Some(&bytes))
			}
		}
	}

	fn u32_at(&self, caller: &wasmtime::Caller<'_, crate::host::HostState>, address: usize) -> u32 {
		self.memory
			.data(caller)
			.get(address..address + 4)
			.map_or(0, |bytes| u32::from_le_bytes(bytes.try_into().unwrap_or_default()))
	}

	fn text_at(&self, caller: &wasmtime::Caller<'_, crate::host::HostState>, address: u32) -> Option<Vec<u8>> {
		(address != 0).then(|| c_text(self.memory.data(caller), address as usize).to_vec())
	}

	fn text_node(
		&self,
		caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
		text: Option<&[u8]>,
	) -> wasmtime::Result<wasmtime::Val> {
		let name = self.name;
		crate::ffi::text_node(caller, text).map_err(|failure| wasmtime::format_err!("{name}: {failure}"))
	}

	fn freed(self, caller: &mut wasmtime::Caller<'_, crate::host::HostState>) -> wasmtime::Result<()> {
		if let Ok(free) = self.instance.get_typed_func::<i32, ()>(&mut *caller, "free") {
			for block in self.allocated {
				free.call(&mut *caller, block)?;
			}
		}
		Ok(())
	}
}

/// A call of a function of a module warp compiled that takes or gives values: each value argument is read out of the
/// program's instance and built in the module's (its texts live in its own memory), the value result the other way
#[cfg(feature = "native")]
pub(crate) fn call_with_values(
	caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
	instance: wasmtime::Instance,
	function: wasmtime::Func,
	arguments: &[wasmtime::Val],
	results: &mut [wasmtime::Val],
) -> wasmtime::Result<()> {
	let module_arguments = arguments
		.iter()
		.map(|argument| copied(caller, argument, None, Some(instance)))
		.collect::<wasmtime::Result<Vec<_>>>()?;
	let mut returned = vec![wasmtime::Val::I32(0); function.ty(&*caller).results().len()];
	function.call(&mut *caller, &module_arguments, &mut returned)?;
	for (argument, module_argument) in arguments.iter().zip(&module_arguments) {
		write_back(caller, argument, module_argument, instance)?;
	}
	for (slot, value) in results.iter_mut().zip(&returned) {
		*slot = copied(caller, value, Some(instance), None)?;
	}
	Ok(())
}

/// A change the module made to its copy of an argument, made to the program's own value: its Node takes the fields of a
/// copy of the changed one, so every reference to it sees the change. The reference itself cannot cross: a Node's texts
/// (field names among them) live in its instance's linear memory, which the other instance reads at the same offsets
#[cfg(feature = "native")]
pub(crate) fn write_back(
	caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
	argument: &wasmtime::Val,
	module_argument: &wasmtime::Val,
	instance: wasmtime::Instance,
) -> wasmtime::Result<()> {
	use wasmtime::AsContextMut;
	let (wasmtime::Val::AnyRef(Some(target)), wasmtime::Val::AnyRef(Some(_))) = (argument, module_argument) else {
		return Ok(());
	};
	let memory = |caller: &mut wasmtime::Caller<'_, crate::host::HostState>, from| {
		export_of(caller, from, "memory")
			.and_then(wasmtime::Extern::into_memory)
			.ok_or_else(|| wasmtime::format_err!("a value's instance exports no memory"))
	};
	let (program_memory, module_memory) = (memory(caller, None)?, memory(caller, Some(instance))?);
	let changed = crate::run::wasm_reader::node_in(module_argument, &mut caller.as_context_mut(), module_memory)
		!= crate::run::wasm_reader::node_in(argument, &mut caller.as_context_mut(), program_memory);
	if !changed {
		return Ok(());
	}
	let wasmtime::Val::AnyRef(Some(changed)) = copied(caller, module_argument, Some(instance), None)? else {
		return Ok(());
	};
	let (Some(target), Some(changed)) = (target.as_struct(&*caller)?, changed.as_struct(&*caller)?) else {
		return Ok(());
	};
	for field in 0..NODE_FIELDS {
		let value = changed.field(&mut *caller, field)?;
		target.set_field(&mut *caller, field, value)?;
	}
	Ok(())
}

/// A value of one instance (None: the program's) as the same value in the other one; a number stays itself
#[cfg(feature = "native")]
pub(crate) fn copied(
	caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
	value: &wasmtime::Val,
	from: Option<wasmtime::Instance>,
	into: Option<wasmtime::Instance>,
) -> wasmtime::Result<wasmtime::Val> {
	use wasmtime::AsContextMut;
	if !matches!(value, wasmtime::Val::AnyRef(_)) {
		return Ok(*value);
	}
	let memory = export_of(caller, from, "memory")
		.and_then(wasmtime::Extern::into_memory)
		.ok_or_else(|| wasmtime::format_err!("a value's instance exports no memory"))?;
	let node = crate::run::wasm_reader::node_in(value, &mut caller.as_context_mut(), memory);
	let value = crate::host::tasks::TaskValue::of(&node).map_err(|failure| wasmtime::format_err!("{failure}"))?;
	let builders = crate::host::tasks::Builders::of(&mut |name| export_of(caller, into, name))
		.map_err(|failure| wasmtime::format_err!("{failure}"))?;
	builders.build(&value, &mut caller.as_context_mut()).map_err(|failure| wasmtime::format_err!("{failure}"))
}

/// The export `name` of the instance (None: the program's)
#[cfg(feature = "native")]
pub(crate) fn export_of(
	caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
	instance: Option<wasmtime::Instance>,
	name: &str,
) -> Option<wasmtime::Extern> {
	match instance {
		Some(instance) => instance.get_export(&mut *caller, name),
		None => caller.get_export(name),
	}
}

/// A count the program passed (a length, a capacity) as a size
#[cfg(feature = "native")]
pub(crate) fn count_of(value: &wasmtime::Val) -> usize {
	value.i32().map(|count| count as u32 as usize).or_else(|| value.i64().map(|count| count as usize)).unwrap_or(0)
}

/// The NUL-terminated bytes at `address` of a linear memory
#[cfg(feature = "native")]
pub(crate) fn c_text(memory: &[u8], address: usize) -> &[u8] {
	let rest = memory.get(address..).unwrap_or_default();
	&rest[..rest.iter().position(|byte| *byte == 0).unwrap_or(rest.len())]
}

/// The run's instance of the module at `path`, one per file however it is named; its own imports (WASI, warp's host
/// words, C libraries, other modules) are linked like a program's
#[cfg(feature = "native")]
pub(crate) fn instance_of(
	caller: &mut wasmtime::Caller<'_, crate::host::HostState>,
	path: &str,
) -> wasmtime::Result<wasmtime::Instance> {
	let file =
		std::fs::canonicalize(path).map_or_else(|_| path.to_string(), |file| file.to_string_lossy().into_owned());
	if let Some(instance) = caller.data().wasm_modules.get(&file) {
		return Ok(*instance);
	}
	let bytes = module_bytes(path).map_err(wasmtime::Error::msg)?;
	let engine = caller.engine().clone();
	let module = wasmtime::Module::new(&engine, &bytes)?;
	let mut linker = wasmtime::Linker::new(&engine);
	crate::run::wasm_reader::link_imports(&mut linker, &engine, &module, crate::run::wasm_reader::Imports::EVERY)
		.map_err(|failure| wasmtime::format_err!("{path}: {failure:#}"))?;
	let instance = linker.instantiate(&mut *caller, &module)?;
	caller.data_mut().wasm_modules.insert(file, instance);
	Ok(instance)
}
