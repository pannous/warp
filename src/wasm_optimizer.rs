use std::io::Write;
use std::path::Path;
use std::process::Command;

/// Optimization mode for WASM output
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OptimizationMode {
	/// No optimization, output raw emitted code
	None,
	/// Basic optimizations (-O1), fast
	Basic,
	/// Standard optimizations (-O2)
	Standard,
	/// Aggressive size optimization (-Oz)
	Size,
	/// Aggressive speed optimization (-O3)
	Speed,
	/// Maximum optimization (-O4)
	Maximum,
}

/// Controls which exports to preserve during tree-shaking
#[derive(Debug, Clone)]
pub enum ExportMode {
	/// Library mode: preserve ALL exports (no tree-shaking of exports)
	Library,
	/// Executable mode: only keep specified entry points, tree-shake everything else
	Executable { entry_points: Vec<String> },
}

impl Default for ExportMode {
	fn default() -> Self {
		ExportMode::Library
	}
}

/// WASM optimizer using binaryen tools (wasm-opt, wasm-metadce)
pub struct WasmOptimizer {
	pub optimization: OptimizationMode,
	pub export_mode: ExportMode,
}

impl Default for WasmOptimizer {
	fn default() -> Self {
		Self {
			optimization: OptimizationMode::Standard,
			export_mode: ExportMode::Library,
		}
	}
}

impl WasmOptimizer {
	pub fn new() -> Self {
		Self::default()
	}

	/// Create optimizer for library output (preserves all exports)
	pub fn library(optimization: OptimizationMode) -> Self {
		Self {
			optimization,
			export_mode: ExportMode::Library,
		}
	}

	/// Create optimizer for executable output (tree-shakes to entry points)
	pub fn executable(optimization: OptimizationMode, entry_points: Vec<String>) -> Self {
		Self {
			optimization,
			export_mode: ExportMode::Executable { entry_points },
		}
	}

	/// Optimize WASM bytes, returns optimized bytes
	pub fn optimize(&self, wasm_bytes: &[u8]) -> Result<Vec<u8>, String> {
		if self.optimization == OptimizationMode::None
			&& matches!(self.export_mode, ExportMode::Library)
		{
			return Ok(wasm_bytes.to_vec());
		}

		// Write input to temp file
		let input_path = std::env::temp_dir().join("wasp_opt_input.wasm");
		let output_path = std::env::temp_dir().join("wasp_opt_output.wasm");

		std::fs::write(&input_path, wasm_bytes)
			.map_err(|e| format!("Failed to write temp input: {}", e))?;

		// Step 1: Tree-shaking with wasm-metadce if in executable mode
		let intermediate_path = if let ExportMode::Executable { ref entry_points } = self.export_mode
		{
			self.run_tree_shaking(&input_path, entry_points)?
		} else {
			input_path.clone()
		};

		// Step 2: Run wasm-opt for optimization
		if self.optimization != OptimizationMode::None {
			self.run_wasm_opt(&intermediate_path, &output_path)?;
		} else {
			std::fs::copy(&intermediate_path, &output_path)
				.map_err(|e| format!("Failed to copy: {}", e))?;
		}

		// Clean up intermediate if created
		if intermediate_path != input_path {
			let _ = std::fs::remove_file(&intermediate_path);
		}
		let _ = std::fs::remove_file(&input_path);

		let result = std::fs::read(&output_path)
			.map_err(|e| format!("Failed to read output: {}", e))?;
		let _ = std::fs::remove_file(&output_path);

		Ok(result)
	}

	/// Run wasm-metadce for tree-shaking, returns path to output
	fn run_tree_shaking(&self, input: &Path, entry_points: &[String]) -> Result<std::path::PathBuf, String> {
		let output = std::env::temp_dir().join("wasp_metadce_output.wasm");
		let graph_path = std::env::temp_dir().join("wasp_roots.json");

		// Build graph JSON for wasm-metadce
		let graph = self.build_roots_graph(entry_points);
		std::fs::write(&graph_path, graph)
			.map_err(|e| format!("Failed to write roots graph: {}", e))?;

		let result = Command::new("wasm-metadce")
			.arg("--enable-gc")
			.arg("--enable-reference-types")
			.arg(input)
			.arg("-f")
			.arg(&graph_path)
			.arg("-o")
			.arg(&output)
			.output()
			.map_err(|e| format!("Failed to run wasm-metadce: {}", e))?;

		let _ = std::fs::remove_file(&graph_path);

		if !result.status.success() {
			let stderr = String::from_utf8_lossy(&result.stderr);
			// wasm-metadce outputs "unused:" lines to stderr but still succeeds
			if !output.exists() {
				return Err(format!("wasm-metadce failed: {}", stderr));
			}
		}

		Ok(output)
	}

	/// Build the roots graph JSON for wasm-metadce
	fn build_roots_graph(&self, entry_points: &[String]) -> String {
		let mut graph = String::from("[\n  {\n    \"name\": \"root\",\n    \"reaches\": [");

		let reaches: Vec<String> = entry_points
			.iter()
			.map(|ep| format!("\"export_{}\"", ep))
			.collect();
		graph.push_str(&reaches.join(", "));
		graph.push_str("],\n    \"root\": true\n  }");

		for ep in entry_points {
			graph.push_str(&format!(
				",\n  {{\n    \"name\": \"export_{}\",\n    \"export\": \"{}\"\n  }}",
				ep, ep
			));
		}

		graph.push_str("\n]\n");
		graph
	}

	/// Run wasm-opt for optimization
	fn run_wasm_opt(&self, input: &Path, output: &Path) -> Result<(), String> {
		let opt_flag = match self.optimization {
			OptimizationMode::None => return Ok(()),
			OptimizationMode::Basic => "-O1",
			OptimizationMode::Standard => "-O2",
			OptimizationMode::Size => "-Oz",
			OptimizationMode::Speed => "-O3",
			OptimizationMode::Maximum => "-O4",
		};

		let result = Command::new("wasm-opt")
			.arg("--enable-gc")
			.arg("--enable-reference-types")
			.arg(opt_flag)
			.arg("--remove-unused-module-elements")
			.arg(input)
			.arg("-o")
			.arg(output)
			.output()
			.map_err(|e| format!("Failed to run wasm-opt: {}", e))?;

		if !result.status.success() {
			let stderr = String::from_utf8_lossy(&result.stderr);
			// Ignore warnings, only fail on actual errors
			if !output.exists() {
				return Err(format!("wasm-opt failed: {}", stderr));
			}
		}

		Ok(())
	}

	/// Optimize and write to file
	pub fn optimize_to_file(&self, wasm_bytes: &[u8], output_path: &Path) -> Result<(), String> {
		let optimized = self.optimize(wasm_bytes)?;
		std::fs::write(output_path, optimized)
			.map_err(|e| format!("Failed to write output: {}", e))?;
		Ok(())
	}

	/// Check if optimization tools are available
	pub fn tools_available() -> bool {
		Command::new("wasm-opt")
			.arg("--version")
			.output()
			.map(|o| o.status.success())
			.unwrap_or(false)
	}

	/// Check if tree-shaking tools are available
	pub fn tree_shaking_available() -> bool {
		Command::new("wasm-metadce")
			.arg("--help")
			.output()
			.map(|o| o.status.success())
			.unwrap_or(false)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_tools_available() {
		assert!(WasmOptimizer::tools_available(), "wasm-opt not found");
		assert!(WasmOptimizer::tree_shaking_available(), "wasm-metadce not found");
	}

	#[test]
	fn test_roots_graph_generation() {
		let optimizer = WasmOptimizer::executable(
			OptimizationMode::Standard,
			vec!["main".to_string(), "init".to_string()],
		);

		if let ExportMode::Executable { ref entry_points } = optimizer.export_mode {
			let graph = optimizer.build_roots_graph(entry_points);
			assert!(graph.contains("\"export\": \"main\""));
			assert!(graph.contains("\"export\": \"init\""));
			assert!(graph.contains("\"root\": true"));
		}
	}
}
