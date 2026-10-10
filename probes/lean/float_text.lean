import WarpTypes.Semantics
open Warp in
#eval [0.5, 0.1, 0.1+0.2, 2.0, 1e20, 1.5e-7, 1.0/3.0, -2.25, 1e300, 123456.789, 1e-5, 0.00001234, 1e15, 999999999999999.9, 5e-324].map floatText
