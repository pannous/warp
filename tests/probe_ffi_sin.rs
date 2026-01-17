// Probe test for FFI sin function via libm
use warp::is;

#[test]
fn test_sin_zero() {
    is!("import sin from 'm'\nsin(0.0)", 0.0);
}

#[test]
fn test_sin_pi_half() {
    // sin(π/2) = 1.0
    is!("import sin from 'm'\nsin(1.5707963267948966)", 1.0);
}

#[test]
fn test_cos_zero() {
    // cos(0) = 1
    is!("import cos from 'm'\ncos(0.0)", 1.0);
}

#[test]
fn test_sqrt_via_ffi() {
    is!("import sqrt from 'm'\nsqrt(4.0)", 2.0);
}
