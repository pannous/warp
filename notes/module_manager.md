# Module manager: the sketch (card module-manager)

samples/modules.warp was this sketch of a module system, from the warp repo. It ended in an uncaught error through
the CLI (card samples-uncaught), so the sample now shows the module forms that work, and the sketch lives here as
input for card module-manager. What it needs, as found on 2026-10-09:
- `module name { … }` blocks, nested ones, and `graphics.colors.red` into them: `module` is an undefined variable.
- `use super.colors` inside a nested module: "module not found: super.colors".
- `use { sin, cos, pi } from math` works but warns "[FFI] No functions found for library 'sin'".
- `use math as m`: `m.sqrt(16)` reads m as the unit metre (DimensionError).
- `use * from utils`: "undefined variable: use".
- `use geometry as geo`, `use platform.windows if os == "windows"`: taken as C libraries ("[FFI] No functions found").
- `def _helper() := ...`: "Unexpected character '.'" (`...` as a placeholder body).

```warp
// Module system in warp
// Simple, explicit imports

// Import entire module
use math

// Import specific items
use { sin, cos, pi } from math

// Import with alias
use geometry as geo
use { Matrix as Mat } from linear_algebra

// Import all (use sparingly)
use * from utils

// Conditional imports
use platform.windows if os == "windows"
use platform.unix if os != "windows"

// Module definition
module shapes {
    // Public by default
    def area_circle(r) := pi * r * r
    def area_rect(w, h) := w * h

    // Private with underscore prefix
    def _helper() := ...

    // Export type
    type Point: { x: float, y: float }
    type Circle: { center: Point, radius: float }
}

// Nested modules
module graphics {
    module colors {
        red = 0xFF0000
        green = 0x00FF00
        blue = 0x0000FF
    }

    module shapes {
        use super.colors
        // ...
    }
}

// Access nested module
color = graphics.colors.red
```
