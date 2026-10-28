//! warp-host's C calls: warp_runtime::c_imports, an import the linker already defines with a fitting type kept
use crate::Host;
pub use warp_runtime::c_imports::{custom_sections, described, CImports};
use wasmtime::{AsContextMut, Linker, Module, Result};

/// Link every C import of `module` that the linker has no fitting definition for, or that `c.described` names
pub fn link_c_imports(
	linker: &mut Linker<Host>,
	store: &mut impl AsContextMut<Data = Host>,
	module: &Module,
	c: &CImports,
) -> Result<()> {
	warp_runtime::c_imports::link_c_imports(linker, module, c, |linker, library, name, func_type| {
		let fitting = linker.get(&mut *store, library, name).ok().and_then(|known| known.ty(&*store).func().cloned());
		fitting.is_some_and(|known| known.matches(func_type))
	})
}
