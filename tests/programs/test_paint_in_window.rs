// card paint-keeps (step a of gpu-functions): a window's paint(shader, …) sends the shader and its values to the paint
// window, which renders them on its own GPU device; no frame of pixels crosses (the 8 MB readback and pipe of
// samples/gpu/gpu_visualizer.warp). No window opens here
#![cfg(feature = "native")]
use warp::graphics::gpu::ShaderValue;
use warp::graphics::paint_window::{read_message, shader_message, Message};

const SHADER: &str = "@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
	return vec4f(at.x / values.width, values.tone, values.levels[1].y, 1.0);
}";
const FULL_HD: (u32, u32) = (1920, 1080);
/// The shader, its values and the marks: a frame of pixels would be 8 MB
const MOST_MESSAGE_BYTES: usize = 1024;

fn values() -> Vec<ShaderValue> {
	let value = |name: &str, floats: Vec<f32>, array| ShaderValue::numbers(name.into(), floats, array);
	vec![
		value("width", vec![64.0], false),
		value("tone", vec![0.25], false),
		value("levels", (0..8).map(|level| level as f32 / 8.0).collect(), true),
	]
}

#[test]
fn a_shader_frame_crosses_to_the_paint_window_whole_and_small() {
	let message = shader_message(FULL_HD.0, FULL_HD.1, SHADER, &values());
	assert!(message.len() < MOST_MESSAGE_BYTES, "{} bytes", message.len());
	let Some(Message::Shader(read)) = read_message(&mut message.as_slice()) else {
		panic!("no shader frame read back")
	};
	assert_eq!((read.width, read.height, read.shader.as_str()), (FULL_HD.0, FULL_HD.1, SHADER));
	let floats = |values: &[ShaderValue]| -> Vec<_> {
		values.iter().map(|value| (value.name.clone(), value.floats.clone(), value.array)).collect()
	};
	assert_eq!(floats(&read.values), floats(&values()));
}

#[test]
fn the_paint_window_renders_what_paint_renders() {
	let Ok(rendered) = warp::graphics::gpu::with_device(|device, queue| {
		let texture = warp::graphics::gpu::render_texture(device, queue, SHADER, 64, 48, &values(), None)?;
		warp::graphics::gpu::image_rgba(device, queue, device.create_command_encoder(&Default::default()), &texture)
	}) else {
		return eprintln!("no GPU here");
	};
	let read_back = warp::graphics::gpu::render_rgba(SHADER, 64, 48, &values()).expect("the shader renders");
	assert_eq!(rendered.expect("the shader renders in a texture"), read_back);
}

#[test]
fn a_shader_that_does_not_compile_fails_before_it_reaches_the_window() {
	if warp::graphics::gpu::with_device(|_, _| ()).is_err() {
		return eprintln!("no GPU here");
	}
	let problem = warp::graphics::gpu::checked_values("@fragment fn main( {", 64, 48, &[]).err();
	assert!(problem.is_some_and(|problem| problem.contains("expected")), "it compiled");
}
