import test from 'ava'

import { Monty, MontySnapshot, MontySyntaxError, MontyRuntimeError } from '../wrapper'

// =============================================================================
// Bug 2: JS bindings panic on OS calls in start()
//
// When Python code triggers OS calls (e.g., Path.exists(), os.getenv()) via
// the start() iterative execution path, the JS bindings panic with:
//   panic!("OS calls are not yet supported in the JS bindings: {function:?}")
// at monty_cls.rs:728-729.
//
// The run() method handles this correctly by returning Err(Error::from_reason(...))
// at monty_cls.rs:273-276, but start() uses progress_to_result() which panics.
//
// Expected: start() should throw MontyRuntimeError, not crash the process.
// =============================================================================

test('bug2: os.getenv via run() returns proper error', (t) => {
  // run() correctly returns an error for OS calls (this is the FIXED path)
  const code = `
import os
os.getenv('HOME')
`
  const m = new Monty(code)
  const error = t.throws(() => m.run())
  // run() correctly wraps this as an error
  t.truthy(error)
})

test('bug2: Path.exists via run() returns proper error', (t) => {
  // run() correctly returns an error for OS calls
  const code = `
from pathlib import Path
Path('/tmp').exists()
`
  const m = new Monty(code)
  const error = t.throws(() => m.run())
  t.truthy(error)
})

// NOTE: The following test demonstrates the actual bug. It is commented out
// because it would crash the Node.js test runner process (panic! kills the process).
//
// Uncomment to verify the bug exists (will crash the test runner):
//
// test('bug2: os call via start() panics instead of throwing', (t) => {
//   // This code calls an external function, then does an OS call.
//   // The external function causes start() to use the iterative path.
//   // When resumed, the OS call hits progress_to_result() which panics.
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
//   // This resume triggers os.getenv('HOME'), which returns OsCall.
//   // progress_to_result() panics instead of returning an error.
//   // BUG: This crashes the Node.js process!
//   t.throws(() => snapshot.resume({ returnValue: 42 }))
// })

// =============================================================================
// Bug 4a: Matrix multiply operator (@) panics the VM
//
// The @ operator compiles to BinaryMatMul opcode but the VM handler is
// `todo!("BinaryMatMul not implemented")` which panics.
//
// Expected: Should throw MontyRuntimeError, not crash the process.
// =============================================================================

// NOTE: This test is commented out because it panics and crashes the process.
//
// test('bug4a: matmul operator panics VM', (t) => {
//   const m = new Monty('1 @ 2')
//   // BUG: This panics with todo!() instead of throwing an error
//   t.throws(() => m.run(), { instanceOf: MontyRuntimeError })
// })

// =============================================================================
// Bug 4b: In-place matrix multiply (@=) panics at compile time
//
// The @= operator hits todo!() in the compiler, panicking during Monty construction.
//
// Expected: Should throw MontySyntaxError, not crash the process.
// =============================================================================

// NOTE: This test is commented out because it panics and crashes the process.
//
// test('bug4b: imatmul operator panics compiler', (t) => {
//   // BUG: This panics during compilation with todo!("InplaceMatMul not yet defined")
//   t.throws(() => new Monty('x = 1\nx @= 2'), { instanceOf: MontySyntaxError })
// })

// =============================================================================
// Bug 4d: raise X from Y silently drops the cause
//
// The parser ignores the 'from Y' part of raise statements.
// This test shows the cause is lost (not a crash, but incorrect behavior).
// =============================================================================

test('bug4d: raise from drops the cause silently', (t) => {
  const code = `
try:
    raise ValueError('effect') from TypeError('cause')
except ValueError as e:
    result = str(e)
result
`
  const m = new Monty(code)
  const result = m.run()
  // The raise works, but the 'from TypeError("cause")' is silently dropped.
  // In CPython, the exception would have a __cause__ attribute.
  t.is(result, 'effect')
  // Bug: No way to access the cause - it was never compiled into the bytecode
})
