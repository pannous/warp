use warp::extensions::print;
use warp::is;

#[test]
fn test_arithmetic() {
	print("Testing basic arithmetic...");
	is!("2+3", 5);
	is!("10-3", 7);
	is!("4*5", 20);
	is!("15/3", 5);
	is!("-2+3", 1);
	is!("2+-3", -1);
	is!("-2+-3", -5);
	is!("-15/3", -5);
	is!("15/-3", -5);
	is!("-15/-3", 5);
	is!("-2*-3", 6);
	is!("-2*3", -6);
	is!("0+0", 0);
	is!("0*5", 0);
	is!("0/1", 0);
	is!("0-5", -5);
	print("✓ Basic arithmetic tests passed");
}
#[test]
fn test_harder_arithmetic() {
	print("Testing harder arithmetic...");
	is!("2+3*4", 14); // precedence
	is!("10-3*2", 4); // precedence
	is!("-2+3*4", 10); // precedence with negative
	// is!("10--3*2", 16); // double negative - TODO: parser interprets -- as decrement operator on literal
	is!("0+3*4", 12); // precedence with zero
	is!("-5*2+3", -7); // negative multiplication first
	is!("2*-3+4", -2); // negative in multiplication
}

#[test]
fn test_power() {
	// eq!(powi(10, 1), 10l);
	// eq!(powi(10, 2), 100l);
	// eq!(powi(10, 3), 1000l);
	// eq!(powi(10, 4), 10000l);
	// eq!(parseLong("8e6"), 8000000l);
	// skip!(
	//
	//     eq!(parseLong("8e-6"), 1.0 / 8000000l);
	// );
	// eq!(parseDouble("8.333e-3"), 0.008333l);
	// eq!(parseDouble("8.333e3"), 8333.0l);
	// eq!(parseDouble("8.333e-3"), 0.008333l);
	// //    eq!(ftoa(8.33333333332248946124e-03), "0.0083");
	// eq!(powi(10, 1), 10l);
	// eq!(powi(10, 2), 100l);
	// eq!(powi(10, 4), 10000l);
	// eq!(powi(2, 2), 4l);
	// eq!(powi(2, 8), 256l);
	// skip!(
	//
	//     eq!(powd(2, -2), 1 / 4.);
	//     eq!(powd(2, -8), 1 / 256.);
	//     eq!(powd(10, -2), 1 / 100.);
	//     eq!(powd(10, -4), 1 / 10000.);
	//     eq!(powd(3,0), 1.);
	//     eq!(powd(3,1), 3.);
	//     eq!(powd(3,2), 9.);
	//     eq!(powd(3,2.1), 10.04510856630514);
	//
	//     //==============================================================================
	//     // MAP TESTS (see map_tests.h);
	//     //==============================================================================
	//
	//     eq!(powd(3.1,2.1), 10.761171606099687);
	// );
	// is!("√3^0", 0.9710078239440918); // very rough power approximation from where?
}

#[test]
#[ignore]
fn test_hyphen_units() {
	//     const char *code = "1900 - 2000 AD";// (easy with units);
	//     assert_analyze(code,"{kind=range type=AD value=(1900,2000)}");
	// todo how does Julia represent 10 ± 2 m/s ?
	is!("1900 - 2000 AD == 1950 AD ± 50", true);
	is!("1900 - 2000 cm == 1950 cm ± 50", true);
	is!("1900 - 2000 cm == 1950 ± 50 cm ", true);
}


#[test]
fn test_variable_minus() {
	is!("a=-1; b=2; b - a", 3); // spaces needed, b-a is kebab-case identifier
}

#[test]
#[ignore]
fn test_hypen_versus_minus() {
	test_variable_minus();
	is!("a-b:2 c-d:4 a-b", 2); // kebab
}

#[test]
fn test_modulo() {
	is!("10007%10000", 7);
	is!("10007.0%10000", 7);
	is!("10007.0%10000.0", 7);
	is!("-10007%10000", -7);
	is!("1%10000", 1);
	is!("9999%10000", 9999);
	is!("10000%10000", 0);
	is!("10001%10000", 1);
	is!("0%10000", 0);
	is!("7%10", 7);
	is!("17%10", 7);
}

#[test]
fn test_simple_variables() {
	is!("x:=42; x", 42);
	is!("x:=10; y:=3; x+y", 13);
	is!("x:=5; x*x", 25);
	is!("x:=-5; x*x", 25);
	is!("x:=-10; y:=3; x+y", -7);
	is!("x:=0; y:=5; x+y", 5);
	is!("x:=-3; y:=-2; x*y", 6);
	is!("x:=0; x*100", 0);
}

#[test]
// #[ignore = "soon"] // needs variable support with mixed types or automatic casting
fn test_modulo_with_variables() {
	is!("10007%10000.0", 7);
	is!("i:=10007;i%10000", 7);
	is!("i:=10007.0;i%10000.0", 7);
	is!("i:=10007.1;i%10000.1", 7);
	is!("i:=-10007;i%10000", -7);
	is!("i:=0;i%10000", 0);
	is!("i:=10000;i%10000", 0);
}

// One of the few tests which can be removed because who will ever change the sin routine?
#[test]
#[ignore = "internal sinus implementation testing - sin/cos/pi not defined"]
fn test_sin() {
	use warp::eq;
	use std::f64::consts::PI as pi;
	fn sin(x: f64) -> f64 { x.sin() }
	fn cos(x: f64) -> f64 { x.cos() }
	eq!(sin(0.), 0.);
	eq!(sin(pi / 2.), 1.);
	eq!(sin(-pi / 2.), -1.);
	eq!(sin(pi), 0.);
	eq!(sin(2. * pi), 0.);
	eq!(sin(3. * pi / 2.), -1.);

	eq!(cos(-pi / 2. + 0.), 0.);
	eq!(cos(0.), 1.);
	eq!(cos(-pi / 2. + pi), 0.);
	eq!(cos(-pi / 2. + 2. * pi), 0.);
	eq!(cos(pi), -1.);
	eq!(cos(-pi), -1.);
}

#[test]
#[ignore]
fn test_primitive_types() {
	is!("double 2", 2);
	is!("float 2", 2);
	is!("int 2", 2);
	is!("long 2", 2);
	is!("8.33333333332248946124e-03", 0);
	is!("8.33333333332248946124e+01", 83);
	is!("S1  = -1.6666", -1);
	is!("double S1  = -1.6666", -1);
	//  is!("double\n" "\tS1  = -1.6666", -1);

	is!("grow(double z):=z*2;grow 5", 10);
	is!("grow(z):=z*2;grow 5", 10);
	is!("int grow(double z):=z*2;grow 5", 10);
	is!("double grow(z):=z*2;grow 5", 10);
	is!("int grow(int z):=z*2;grow 5", 10);
	is!("double grow(int z):=z*2;grow 5", 10);
	is!("double\n\tS1  = -1.66666666666666324348e01, /* 0xBFC55555, 0x55555549 */\n\tS2  =  8.33333333332248946124e03, /* 0x3F811111, 0x1110F8A6 */\n\nS1", -16);
	is!("double\n\tS1  = -1.66666666666666324348e01, /* 0xBFC55555, 0x55555549 */\n\tS2  =  8.33333333332248946124e01, /* 0x3F811111, 0x1110F8A6 */\n\nS2", 83);
	// eq!(ftoa(8.33333333332248946124e-03), "0.0083");
	//  eq!(ftoa2(8.33333333332248946124e-03), "8.333E-3");
	is!("S1 = -1.66666666666666324348e-01;S1*100", -16);
	is!("S1 = 8.33333333332248946124e-03;S1*1000", 8);
	is!("(2,4) == (2,4)", 1); // todo: array creation/ comparison
	is!("(float 2, int 4.3)  == 2,4", 1); //  PRECEDENCE needs to be in valueNode :(
	is!("float 2, int 4.3  == 2,4", 1); //  PRECEDENCE needs to be in valueNode :(
	                                 //  float  2, ( int ==( 4.3 2)), 4
}

#[test]
fn test_logarithm_in_runtime() {

	// float
	// let ℯ = 2.7182818284590;
	//	eq!(ln(0),-∞);
	// eq!(log(100000),5.);
	// eq!(log(10),1.);
	// eq!(log(1),0.);
	// eq!(ln(ℯ*ℯ),2.);
	// eq!(ln(1),0.);
	// eq!(ln(ℯ),1.);
}

#[test]
#[ignore]
fn test_sinus_wasp_import() {
	// using sin.wasp, not sin.wasm
	// todo: compile and reuse sin.wasm if unmodified
	is!("use sin;sin π/2", 1);
	is!("use sin;sin π", 0);
	is!("use sin;sin 3*π/2", -1);
	is!("use sin;sin 2π", 0);
	is!("use sin;sin -π/2", -1);
}

#[test]
#[ignore]
fn test_units() {
	is!("1 m + 1km", 1001); // todo m
}

#[test]
#[ignore]
fn test_eval() {
	is!("√4", 2);
}

#[test]
fn test_runtime_equality() {
	is!("3==2+1", true);
	is!("3.1==3.1", true); // Obviously, we need to select the correct equality operator per type.
	is!("3*452==452*3", 1);
	is!("3*13==14*3", 0);
	is!("3*13==14*3", 0);
	is!("-3==-3", 1);
	is!("0==0", 1);
	is!("-5+3==-2", 1);
	is!("0.0==0", 1);
}

#[test]
fn test_runtime_equality_autocast() {
	// A very general autocast mechanism works pretty well in C++. see there for inspiration.
	is!("3==3.0", true);
	is!("-3==-3.0", true);
	is!("0==0.0", true);
	/* if (node.length == 2) {  // binary operator would be our Key() node
        Node lhs = node.children[0]; //["lhs"];
        Node rhs = node.children[1]; //["rhs"];
        const Code &lhs_code = emitExpression(lhs, context);
        Type lhs_type = last_type;
        arg_type = last_type; 
        if (isGeneric(last_type))
            arg_type = last_type.generics.value_type;
        const Code &rhs_code = emitExpression(rhs, context);
        Type rhs_type = last_type;
        Type common_type = commonType(lhs_type, rhs_type, name); // 3.1 + 3 => 6.1 etc, -1/6 => float
        bool same_domain = common_type != none; // todo: only some operators * / + - only sometimes autocast!
        code.push(lhs_code); // might be empty ok
        if (same_domain)
            code.add(cast(lhs_type, common_type));
        code.push(rhs_code); // might be empty ok
        if (name == "#") // todo unhack!!
            code.add(cast(rhs_type, int32t)); // index operator, cast to int32
        else if (same_domain)
            code.add(cast(rhs_type, common_type));
        if (common_type != void_block)
            last_type = common_type;
        else last_type = rhs_type;
    */
}

#[test]
fn test_ternary_with_comparison() {
	is!("(1<2)?10:255", 10);
	is!("(1>2)?10:255", 255);
	is!("(-5<0)?1:0", 1);
	is!("(0<1)?-10:-20", -10);
	is!("(0==0)?5:10", 5);
	is!("(-3<-5)?1:0", 0);
}

#[test]
fn test_if_then_else() {
	is!("if 1 then 2 else 3", 2);
	is!("if 0 then 2 else 3", 3);
	is!("if 1<2 then 10 else 255", 10);
	is!("if 1>2 then 10 else 255", 255);
	is!("if -1 then 5 else 10", 5);
	is!("if -5<0 then -10 else -20", -10);
	is!("if 0==0 then 1 else 0", 1);
}

#[test]
fn test_if_block_syntax() {
	is!("if 1 { 2 }", 2);
	is!("if 0 { 2 }", 0); // no else branch returns 0
	is!("if 1<2 { 10 }", 10);
	is!("if 1>2 { 10 }", 0); // no else branch returns 0
	is!("if 1 { 2 } else { 3 }", 2);
	is!("if 0 { 2 } else { 3 }", 3);
	is!("if 1<2 { 10 } else { 255 }", 10);
	is!("if 1>2 { 10 } else { 255 }", 255);
	is!("if -1 { -5 } else { -10 }", -5);
	is!("if -5<0 { 0 }", 0);
	is!("if 0 { -1 }", 0);
}

#[test]
fn test_while_loop() {
	// Simple countdown: while x > 0 { x = x - 1 } returns 0
	is!("x:=3; while x>2 { x -= 1 }", 2);
	// Alternative syntax: while x > 0 do x = x - 1
	is!("x:=3; while x>0 do x = x - 1", 0);
	is!("x:=-3; while x<0 { x += 1 }", 0);
	is!("x:=0; while x<5 { x += 1 }", 5);
}

#[test]
fn test_compound_assignment() {
	// Basic compound assignments
	is!("x:=10; x += 5; x", 15);
	is!("x:=10; x -= 3; x", 7);
	is!("x:=10; x *= 2; x", 20);
	is!("x:=10; x /= 2; x", 5);
	is!("x:=10; x %= 3; x", 1);
	// In expressions
	is!("x:=5; x += 3", 8);  // returns the new value
	// Chained with while
	is!("x:=0; i:=3; while i>0 { x += i; i -= 1 }; x", 6); // 3+2+1=6
	is!("x:=-10; x += 5; x", -5);
	is!("x:=-10; x -= 3; x", -13);
	is!("x:=-10; x *= 2; x", -20);
	is!("x:=-10; x /= 2; x", -5);
	is!("x:=0; x += 5; x", 5);
	is!("x:=0; x *= 5; x", 0);
}

#[test]
#[ignore = "soon"]
fn test_absolute_value_arithmetic() {
	is!("‖3‖-1", 2);
}

#[test]
#[ignore]
fn test_fraction_multiplication() {
	is!("⅓9", 3);
}

#[test]
#[ignore]
fn test_superscript_powers() {
	is!("3⁴", 81);
}

// DONE: type upgrading and global keyword implementation
#[test]
fn test_global_with_pi() {
	use std::f64::consts::PI;
	is!("global x=1+π", 1.0 + PI);
	is!("global x=1+π;x+2", 3.0 + PI); // x = 1+π ≈ 4.14, then x+2 ≈ 6.14 = 3+π
	is!("pi", PI);       // pi is alias for π
	is!("1+pi", 1.0 + PI);
}

#[test]
fn test_sqrt_alias() {
	is!("sqrt 9", 3); // be type invariant:
	is!("sqrt 9.0", 3);
	is!("sqrt 2", std::f64::consts::SQRT_2);
	is!("sqrt 0", 0);
	is!("sqrt 1", 1);
	is!("sqrt 4", 2);
}

#[test]
fn test_abs_alias() {
	is!("abs -3", 3);
	is!("abs 3", 3);
	is!("abs -3.14", 3.14);
	is!("abs 3.14", 3.14);
	is!("abs 0", 0);
	is!("abs -0", 0);
	is!("abs -1", 1);
}
