wit_bindgen::generate!({ world: "demo", path: "wit" });

use exports::warp::demo::text_tools::{Guest as TextTools, Shape, Stats};

struct Demo;

impl Guest for Demo {
	fn fib(n: u32) -> u64 {
		(0..n).fold((0u64, 1u64), |(a, b), _| (b, a + b)).0
	}
}

impl TextTools for Demo {
	fn words(text: String) -> Vec<String> {
		text.split_whitespace().map(str::to_string).collect()
	}
	fn stats_of(text: String) -> Stats {
		Stats { words: text.split_whitespace().count() as u32, letters: text.chars().filter(|c| c.is_alphabetic()).count() as u32 }
	}
	fn sides(of: Shape) -> u32 {
		match of { Shape::Circle => 0, Shape::Square => 4 }
	}
	fn checked_sqrt(x: f64) -> Result<f64, String> {
		if x < 0.0 { Err(format!("no real square root of {x}")) } else { Ok(x.sqrt()) }
	}
}

export!(Demo);
