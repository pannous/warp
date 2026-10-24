// a plain native .obj parser (what a library like tobj does at its core): v and f lines, faces fanned into triangles
use std::time::Instant;
fn main() {
	let text = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
	let started = Instant::now();
	let (mut points, mut corners) = (Vec::<[f32; 3]>::new(), Vec::<[f32; 3]>::new());
	for line in text.lines() {
		let mut words = line.split_whitespace();
		match words.next() {
			Some("v") => { let mut xyz = words.map(|word| word.parse::<f32>().unwrap()); points.push([xyz.next().unwrap(), xyz.next().unwrap(), xyz.next().unwrap()]); }
			Some("f") => {
				let face: Vec<[f32; 3]> = words.map(|word| points[word.split('/').next().unwrap().parse::<usize>().unwrap() - 1]).collect();
				for i in 1..face.len() - 1 { corners.extend([face[0], face[i], face[i + 1]]); }
			}
			_ => {}
		}
	}
	println!("{} triangles in {:.2} ms", corners.len() / 3, started.elapsed().as_secs_f64() * 1000.0);
}
