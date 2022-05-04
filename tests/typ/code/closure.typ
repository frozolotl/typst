// Test closures.
// Ref: false

---
// Don't parse closure directly in content.
// Ref: true

#let x = "x"

// Should output `x => y`.
#x => y

---
// Basic closure without captures.
{
  let adder = (x, y) => x + y
  test(adder(2, 3), 5)
}

---
// Pass closure as argument and return closure.
// Also uses shorthand syntax for a single argument.
{
  let chain = (f, g) => (x) => f(g(x))
  let f = x => x + 1
  let g = x => 2 * x
  let h = chain(f, g)
  test(h(2), 5)
}

---
// Capture environment.
{
  let mark = "?"
  let greet = {
    let hi = "Hi"
    name => {
        hi + ", " + name + mark
    }
  }

  test(greet("Typst"), "Hi, Typst?")

  mark = "!"
  test(greet("Typst"), "Hi, Typst!")
}

---
// Redefined variable.
{
  let x = 1
  let f() = {
    let x = x + 2
    x
  }
  test(f(), 3)
}

---
// Import bindings.
{
  let b = "target.typ"
  let f() = {
    import b from b
    b
  }
  test(f(), 1)
}

---
// For loop bindings.
{
  let v = (1, 2, 3)
  let s = 0
  let f() = {
    for v in v { s += v }
  }
  f()
  test(s, 6)
}

---
// Let + closure bindings.
{
  let g = "hi"
  let f() = {
    let g() = "bye"
    g()
  }
  test(f(), "bye")
}

---
// Parameter bindings.
{
  let x = 5
  let g() = {
    let f(x, y: x) = x + y
    f
  }

  test(g()(8), 13)
}

---
// Don't leak environment.
{
  // Error: 16-17 unknown variable
  let func() = x
  let x = "hi"
  func()
}

---
// Too few arguments.
{
  let types(x, y) = "[" + type(x) + ", " + type(y) + "]"
  test(types(14%, 12pt), "[ratio, length]")

  // Error: 13-21 missing argument: y
  test(types("nope"), "[string, none]")
}

---
// Too many arguments.
{
  let f(x) = x + 1

  // Error: 8-13 unexpected argument
  f(1, "two", () => x)
}

---
// Named arguments.
{
  let greet(name, birthday: false) = {
    if birthday { "Happy Birthday, " } else { "Hey, " } + name + "!"
  }

  test(greet("Typst"), "Hey, Typst!")
  test(greet("Typst", birthday: true), "Happy Birthday, Typst!")

  // Error: 23-35 unexpected argument
  test(greet("Typst", whatever: 10))
}

---
// Error: 6-16 expected identifier, named pair or argument sink
{(a, "named": b) => none}

---
// Error: 10-15 expected identifier
#let foo("key": b) = key

---
// Error: 10-14 expected identifier
#let foo(none: b) = key
