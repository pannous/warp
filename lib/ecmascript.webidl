// ECMAScript's own globals as WebIDL, written by hand from ECMA-262 (no spec publishes them as IDL): the namespaces
// warp checks `use js` calls with beside web.webidl's, so `Math.nonsense(2)` is refused at compile time.

// ECMA-262 21.3 The Math Object
namespace Math {
  const unrestricted double E = 2.718281828459045;
  const unrestricted double LN10 = 2.302585092994046;
  const unrestricted double LN2 = 0.6931471805599453;
  const unrestricted double LOG10E = 0.4342944819032518;
  const unrestricted double LOG2E = 1.4426950408889634;
  const unrestricted double PI = 3.141592653589793;
  const unrestricted double SQRT1_2 = 0.7071067811865476;
  const unrestricted double SQRT2 = 1.4142135623730951;
  unrestricted double abs(unrestricted double x);
  unrestricted double acos(unrestricted double x);
  unrestricted double acosh(unrestricted double x);
  unrestricted double asin(unrestricted double x);
  unrestricted double asinh(unrestricted double x);
  unrestricted double atan(unrestricted double x);
  unrestricted double atanh(unrestricted double x);
  unrestricted double atan2(unrestricted double y, unrestricted double x);
  unrestricted double cbrt(unrestricted double x);
  unrestricted double ceil(unrestricted double x);
  unrestricted double clz32(unrestricted double x);
  unrestricted double cos(unrestricted double x);
  unrestricted double cosh(unrestricted double x);
  unrestricted double exp(unrestricted double x);
  unrestricted double expm1(unrestricted double x);
  unrestricted double floor(unrestricted double x);
  unrestricted double fround(unrestricted double x);
  unrestricted double hypot(unrestricted double... values);
  unrestricted double imul(unrestricted double x, unrestricted double y);
  unrestricted double log(unrestricted double x);
  unrestricted double log1p(unrestricted double x);
  unrestricted double log10(unrestricted double x);
  unrestricted double log2(unrestricted double x);
  unrestricted double max(unrestricted double... values);
  unrestricted double min(unrestricted double... values);
  unrestricted double pow(unrestricted double base, unrestricted double exponent);
  unrestricted double random();
  unrestricted double round(unrestricted double x);
  unrestricted double sign(unrestricted double x);
  unrestricted double sin(unrestricted double x);
  unrestricted double sinh(unrestricted double x);
  unrestricted double sqrt(unrestricted double x);
  unrestricted double sumPrecise(any items);
  unrestricted double tan(unrestricted double x);
  unrestricted double tanh(unrestricted double x);
  unrestricted double trunc(unrestricted double x);
};

// ECMA-262 25.5 The JSON Object
namespace JSON {
  any parse(DOMString text, optional any reviver);
  DOMString stringify(any value, optional any replacer, optional any space);
  any rawJSON(DOMString text);
  boolean isRawJSON(any value);
};

// ECMA-262 28.1 The Reflect Object
namespace Reflect {
  any apply(any target, any thisArgument, any argumentsList);
  any construct(any target, any argumentsList, optional any newTarget);
  boolean defineProperty(any target, any propertyKey, any attributes);
  boolean deleteProperty(any target, any propertyKey);
  any get(any target, any propertyKey, optional any receiver);
  any getOwnPropertyDescriptor(any target, any propertyKey);
  any getPrototypeOf(any target);
  boolean has(any target, any propertyKey);
  boolean isExtensible(any target);
  any ownKeys(any target);
  boolean preventExtensions(any target);
  boolean set(any target, any propertyKey, any V, optional any receiver);
  boolean setPrototypeOf(any target, any proto);
};
