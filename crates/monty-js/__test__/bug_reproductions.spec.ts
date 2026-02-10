import test from 'ava'

import { Monty, MontySnapshot, MontyRuntimeError } from '../wrapper'

// =============================================================================
// Bug 2: JS bindings panic on OS calls in start()
//
// When Python code triggers OS calls (e.g., os.getenv()) via the start()
// iterative execution path, the JS bindings call panic!() in
// progress_to_result() at monty_cls.rs:725-730.
//
// The run() method handles OS calls correctly by returning Error::from_reason(),
// but start() uses progress_to_result() which panics, crashing the Node process.
//
// Expected: start() should throw an error, not crash the process.
// =============================================================================

test('bug2: os.getenv via run() returns proper error (not panic)', (t) => {
  // run() correctly returns an error for OS calls — this is the FIXED path.
  const code = `
import os
os.getenv('HOME')
`
  const m = new Monty(code)
  const error = t.throws(() => m.run())
  t.truthy(error, 'run() should return an error for OS calls without os_access')
})

// The start() path panics instead of returning an error. This test would crash
// the Node.js test runner if uncommented:
//
// test('bug2: os call via start() panics instead of throwing', (t) => {
//   // Use an external function to force start() into the iterative path,
//   // then trigger an OS call on resume.
//   const code = `
// import os
// x = func()
// os.getenv('HOME')
// `
//   const m = new Monty(code, { externalFunctions: ['func'] })
//   const progress = m.start()
//   t.true(progress instanceof MontySnapshot)
//   const snapshot = progress as MontySnapshot
//
//   // BUG: This crashes the Node.js process with:
//   //   panic!("OS calls are not yet supported in the JS bindings: Getenv")
//   // It should throw MontyRuntimeError instead.
//   t.throws(() => snapshot.resume({ returnValue: 42 }))
// })

// =============================================================================
// Bug 4a: Matrix multiply operator (@) panics the VM
//
// The @ operator compiles to BinaryMatMul opcode, but the VM handler is
// todo!("BinaryMatMul not implemented") which panics, crashing the process.
//
// Expected: Should throw MontyRuntimeError with TypeError.
// =============================================================================

// Uncomment to verify — this crashes the test runner:
//
// test('bug4a: matmul operator should throw, not crash', (t) => {
//   const m = new Monty('1 @ 2')
//   // BUG: panics with todo!() instead of throwing MontyRuntimeError
//   t.throws(() => m.run(), { instanceOf: MontyRuntimeError })
// })

// =============================================================================
// Bug 4b: In-place matrix multiply (@=) panics at compile time
//
// Expected: Should throw MontySyntaxError, not crash.
// =============================================================================

// Uncomment to verify — this crashes the test runner:
//
// test('bug4b: imatmul operator should throw, not crash', (t) => {
//   // BUG: panics during compilation with todo!("InplaceMatMul not yet defined")
//   t.throws(() => new Monty('x = 1\nx @= 2'))
// })

// =============================================================================
// Bug 4d: raise X from Y silently drops the cause
//
// The parser ignores the 'from Y' part of raise statements. In CPython, the
// exception would have __cause__ set. In Monty, it's silently dropped.
// =============================================================================

test('bug4d: raise from drops the cause silently', (t) => {
  // These two programs should produce DIFFERENT exceptions in CPython
  // (one has __cause__ set, the other doesn't). In Monty, they're identical.
  const withCause = `
try:
    raise ValueError('effect') from TypeError('cause')
except ValueError as e:
    str(e)
`
  const withoutCause = `
try:
    raise ValueError('effect')
except ValueError as e:
    str(e)
`
  const m1 = new Monty(withCause)
  const m2 = new Monty(withoutCause)
  const r1 = m1.run()
  const r2 = m2.run()

  // Both return 'effect' — the cause was silently dropped
  t.is(r1, 'effect')
  t.is(r2, 'effect')
  // Bug: there's no way to distinguish these — the cause was never stored
})
