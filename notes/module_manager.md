# Module manager: the sketch (card module-manager)

The module system sketched below is input for card module-manager (samples/modules/modules.warp shows the forms that
work). Working: `use math`, `use { sin, cos } from math`, `use math as m; m.sqrt(16)`. Missing (checked 2026-10-17):
- `module name { … }` blocks, nested ones, and `graphics.colors.red` into them (`module colors { red = 1 };
  colors.red` is "no field red").
- `use super.colors` inside a nested module.
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
