//! What a painted pixel value shows, natively and in the browser: shared by paint and the shaders of painted functions
//! (lowering/foreign/gpu_maps.rs)

/// The gray levels of a nonzero pixel and of a zero one, as the playground draws them
pub const INK: u8 = 29;
pub const PAPER: u8 = 250;
/// A pixel value from here on carries an alpha byte, 0xAARRGGBB: it is that color (lib/draw.warp), any smaller nonzero
/// value is ink (playground.js COLOR_FROM)
pub const COLOR_FROM: u64 = 1 << 24;
