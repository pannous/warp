// card gpu-draw: `use draw` draws its shapes with a shader into a canvas texture that stays on the GPU
// (src/graphics/gpu_canvas.rs); WARP_DRAW=gpu and WARP_DRAW=cpu paint the same picture, show() sends shapes, not pixels
#![cfg(feature = "native")]
use super::test_paint_lists::painted_rows_by;
use warp::graphics::paint::DRAW_VARIABLE;
use warp::graphics::paint_window::{read_message, shapes_message, Message};

const SCENE: &str = "use draw
canvas(64, 48)
clear(white)
rect(2.5, 3, 10, 7.5, red)
circle(30, 20, 9.5, blue)
circle(40.3, 25.7, 6, with_alpha(green, 0.5))
dot(60, 40, black)
line(0, 47, 63, 30, purple)
rect(50, -5, 30, 12, with_alpha(orange, 0.25))
show()";

fn drawn_by(choice: &str) -> std::process::Command {
	let mut command = crate::common::warp_command();
	command.env(DRAW_VARIABLE, choice);
	command
}

fn has_gpu() -> bool {
	warp::graphics::gpu::with_device(|_, _| ()).is_ok()
}

#[test]
fn shapes_cross_to_the_paint_window_whole() {
	let shapes: Vec<u8> = (0..2 * warp::graphics::gpu_canvas::SHAPE_BYTES as u8).collect();
	let message = shapes_message(64, 48, &shapes);
	match read_message(&mut message.as_slice()) {
		Some(Message::Shapes(read)) => assert_eq!((read.width, read.height, read.shapes), (64, 48, shapes)),
		_ => panic!("no shapes read back"),
	}
}

#[test]
fn gpu_draws_what_cpu_draws() {
	if !has_gpu() {
		return crate::common::announce_skip("a WebGPU adapter", module_path!());
	}
	let on_cpu = painted_rows_by(drawn_by("cpu"), SCENE, "draw-cpu");
	assert_eq!(painted_rows_by(drawn_by("gpu"), SCENE, "draw-gpu"), on_cpu);
}

#[test]
fn an_unknown_draw_choice_warns() {
	let run = drawn_by("quantum").args(["--no-ask", "eval", "1"]).output().unwrap();
	assert!(String::from_utf8_lossy(&run.stderr).contains("WARP_DRAW=quantum is neither"), "{run:?}");
}
