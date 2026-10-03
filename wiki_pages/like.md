# like

`like` declares that values of one type may be used wherever another type is expected:

```wasp
class photo{width:int height:int}
class image{width:int height:int}
image like photo

area(p:photo) := p.width * p.height
area(image{width:3 height:4})   // 12
```

## Duck typing, by design

Wasp judges a value by what is done with it. How a typed parameter `p:photo` treats its argument depends on how much
the compiler knows about that argument:

| argument | result |
|---|---|
| a `photo` | accepted |
| a value of **unknown** type: an untyped parameter passed on, a written map `{width:3 height:4}` | **duck typed**: accepted, and judged by its uses. A field it lacks is a loud runtime error: `no field height` |
| a value of a **known other** type: `image{…}` | compile error that teaches `like`: `image is not a photo; fix: declare image like photo to use it as one` |
| a known non-object: `3`, `"x"` | compile error: `area needs a photo for parameter p, got 3 (an Int)` |

So unknown values get the benefit of the doubt, as in Python: untyped code just runs. Two *known* types never mix
silently, even when one has all the fields of the other: that they are interchangeable is a decision, stated once, in
one line.

## A promise, not a conversion

`image like photo` changes no value. The image stays an image and is still judged by its uses:

```wasp
class photo{width:int height:int}
class thumb{width:int}
thumb like photo

keep(p:photo) := p.width
keep(thumb{width:3})            // 3
area(p:photo) := p.width * p.height
area(thumb{width:3})            // error: no field height
```

## Rules

- `like` relates two declared types (`class`, `type`, `struct`): `banana like photo` without `class banana` is an error.
- It goes one way: `image like photo` lets an image stand for a photo, not a photo for an image.
- A type may be like several types (`image like photo; image like page`).
- Likeness chains: `thumb like image; image like photo` lets a thumb stand for a photo.
- The declaration may come anywhere in the program, before or after its uses.

## Why a keyword

The alternatives were `as` (reads as a conversion, which this is not) and `is` (already the type test `x is photo`).
`like` says what is meant: treat it like one.

See also: matching by type name (`to keep a photo: …`), traits (`class dot{x:int} is Comparable`).
