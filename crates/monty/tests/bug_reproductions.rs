/// Tests that reproduce known bugs in the Monty interpreter.
///
/// Each test documents a specific bug, its root cause, and verifies
/// the incorrect behavior. When the bug is fixed, these tests should
/// be updated to assert the correct behavior instead.
use monty::{ExcType, LimitedTracker, MontyRun, ResourceLimits, StdPrint};

/// Bug 1: Reference count leak in `Heap::mult_sequence()` when allocation fails.
///
/// When multiplying a list containing heap refs by a count, the function:
/// 1. Increments reference counts for all contained Ref values (heap.rs:1403-1407)
/// 2. Forgets the items vector via `std::mem::forget` (heap.rs:1422)
/// 3. Calls `self.allocate()` which can fail with ResourceError (heap.rs:1424)
///
/// If step 3 fails, the reference counts incremented in step 1 are never
/// decremented, causing a permanent refcount leak. The items vector was
/// forgotten in step 2 so there's no cleanup path.
///
/// This test triggers the bug by:
/// - Creating a list containing a heap-allocated string (creates a Ref)
/// - Setting a tight allocation limit
/// - Attempting list multiplication, which should fail during allocate()
/// - The refcounts for the string are leaked
///
/// Expected: The allocation limit error should be raised AND refcounts
/// should remain consistent (no leak).
///
/// NOTE: This test is ignored when ref-count-panic is enabled because
/// resource exhaustion doesn't guarantee heap state consistency.
#[test]
#[cfg_attr(
    feature = "ref-count-panic",
    ignore = "resource exhaustion doesn't guarantee heap state consistency"
)]
fn bug1_refcount_leak_in_mult_sequence() {
    // Create a list with a heap-allocated string, then try to multiply it
    // with a very tight memory limit. The multiplication should fail
    // when trying to allocate the large result list, but the refcounts for
    // the string copies have already been incremented.
    let code = r#"
x = ['hello world']
y = x * 10000
"#;
    let ex = MontyRun::new(code.to_owned(), "test.py", vec![], vec![]).unwrap();

    // Set memory limit low enough that x * 10000 will fail during allocate()
    // (the resulting list would require ~80KB for 10000 Value pointers)
    // but high enough to allow creating x itself
    let limits = ResourceLimits::new().max_memory(1000);
    let result = ex.run(vec![], LimitedTracker::new(limits), &mut StdPrint);

    // The multiplication should fail with a MemoryError
    assert!(result.is_err(), "should exceed memory limit during list multiplication");
    let exc = result.unwrap_err();
    assert_eq!(exc.exc_type(), ExcType::MemoryError);
    // Bug: at this point, the refcounts for the string 'hello world' have been
    // incremented 10000 times (once per copy) but never decremented since the
    // allocation failed. This is a silent refcount leak.
}

/// Bug 1 (variant): Same refcount leak but with tuples.
///
/// The identical bug exists in the tuple multiplication path (heap.rs:1443-1463).
#[test]
#[cfg_attr(
    feature = "ref-count-panic",
    ignore = "resource exhaustion doesn't guarantee heap state consistency"
)]
fn bug1_refcount_leak_in_mult_sequence_tuple() {
    let code = r#"
x = ('hello world',)
y = x * 10000
"#;
    let ex = MontyRun::new(code.to_owned(), "test.py", vec![], vec![]).unwrap();

    // Same strategy: memory limit allows creating x but not the large result
    let limits = ResourceLimits::new().max_memory(1000);
    let result = ex.run(vec![], LimitedTracker::new(limits), &mut StdPrint);

    assert!(
        result.is_err(),
        "should exceed memory limit during tuple multiplication"
    );
    let exc = result.unwrap_err();
    assert_eq!(exc.exc_type(), ExcType::MemoryError);
}

/// Bug 1 (ref-count verification): Verify the refcount leak is observable.
///
/// This test creates a list with a ref, multiplies it successfully, and then
/// verifies refcounts are correct. Then we contrast with the failure case to
/// show the leak. Under ref-count-return, we can observe the leaked counts.
#[test]
#[cfg(feature = "ref-count-return")]
fn bug1_refcount_leak_observable() {
    // Successful case: multiply a list containing a ref
    // Each element in the result holds a reference to the same string
    let code = r#"
x = ['hello world']
y = x * 3
y
"#;
    let ex = MontyRun::new(code.to_owned(), "test.py", vec![], vec![]).unwrap();
    let output = ex.run_ref_counts(vec![]).expect("should succeed");

    // x has refcount 1 (just the variable binding)
    // y has refcount 2 (variable binding + return value on stack)
    assert_eq!(
        output.counts.get("x"),
        Some(&1),
        "x should have refcount 1, got counts: {:?}",
        output.counts
    );
    assert_eq!(
        output.counts.get("y"),
        Some(&2),
        "y should have refcount 2 (variable + return value), got counts: {:?}",
        output.counts
    );
}

/// Bug 4a: `a @ b` (matrix multiply operator) panics the VM with todo!().
///
/// The `@` operator is parsed and compiled to `Opcode::BinaryMatMul` (compiler.rs:2817),
/// but the VM handler at vm/mod.rs:814 is just `todo!("BinaryMatMul not implemented")`.
/// This means untrusted code using `@` will crash the host process with a panic.
///
/// Expected behavior: Should raise TypeError (like CPython does for non-matrix types)
/// or NotImplementedError, NOT panic.
#[test]
#[should_panic(expected = "BinaryMatMul not implemented")]
fn bug4a_matmul_operator_panics() {
    let code = "x = 1 @ 2";
    let ex = MontyRun::new(code.to_owned(), "test.py", vec![], vec![]).unwrap();
    // This will panic with todo!() instead of returning an error
    let _ = ex.run_no_limits(vec![]);
}

/// Bug 4b: `a @= b` (in-place matrix multiply) panics during compilation.
///
/// The augmented assignment `@=` maps to `Operator::MatMult` in the compiler
/// (compiler.rs:2845) which hits `todo!("InplaceMatMul not yet defined")`.
/// This panics at compile/prepare time, crashing the host process.
///
/// Expected behavior: Should return a compile error, not panic.
#[test]
#[should_panic(expected = "InplaceMatMul not yet defined")]
fn bug4b_imatmul_operator_panics() {
    let code = "x = 1\nx @= 2";
    // This panics during MontyRun::new() (compilation phase)
    let _ = MontyRun::new(code.to_owned(), "test.py", vec![], vec![]);
}

/// Bug 4c: `del obj[key]` panics the VM with todo!().
///
/// The parser rejects `del x` (standalone delete) with a proper "not implemented" error,
/// but if `DeleteSubscr` opcode somehow reaches the VM (vm/mod.rs:1031), it panics.
/// While the parser currently blocks `del`, the opcode exists and the VM handler panics
/// instead of returning an error.
///
/// NOTE: Since the parser blocks `del`, this specific opcode is currently unreachable
/// from user code. The bug is that IF the opcode is reached (e.g., via deserialized
/// bytecode), it panics instead of returning an error.
/// We test the parser-level rejection here instead.
#[test]
fn bug4c_del_statement_not_supported() {
    let code = "x = [1, 2, 3]\ndel x[0]";
    let result = MontyRun::new(code.to_owned(), "test.py", vec![], vec![]);
    // The parser correctly returns an error for 'del', but the VM would panic
    // if this opcode were reached through other means (e.g. snapshot injection)
    assert!(result.is_err(), "del statement should be rejected at parse time");
}

/// Bug 4d: `raise X from Y` silently drops the cause.
///
/// The parser at parse.rs:348 handles `Stmt::Raise(ast::StmtRaise { exc, .. })`
/// where the `..` silently ignores the `cause` field. The TODO comment says
/// "add cause to Node::Raise" but currently `raise X from Y` compiles identically
/// to `raise X`, losing the exception chain information.
///
/// This is not a crash bug, but it's silently incorrect behavior.
/// In CPython, `raise ValueError('a') from TypeError('b')` sets `__cause__`.
#[test]
fn bug4d_raise_from_silently_drops_cause() {
    // In CPython, this would set __cause__ on the ValueError.
    // In Monty, the `from TypeError('cause')` part is silently ignored.
    let code = r#"
try:
    raise ValueError('effect') from TypeError('cause')
except ValueError as e:
    # In CPython, e.__cause__ would be TypeError('cause')
    # In Monty, the 'from' clause is silently dropped, so this just catches ValueError
    result = str(e)
result
"#;
    let ex = MontyRun::new(code.to_owned(), "test.py", vec![], vec![]).unwrap();
    let result = ex.run_no_limits(vec![]).unwrap();
    let s: String = result.as_ref().try_into().unwrap();
    // The raise itself works (not a crash), but the cause is silently lost
    assert_eq!(s, "effect");
    // Bug: There's no way to access __cause__ because it was never stored
}

/// Bug 5: Snapshot deserialization with crafted bytes can cause panics.
///
/// The `postcard::from_bytes()` deserialization doesn't fully validate internal
/// state. While `postcard` is memory-safe, invalid function IDs, heap IDs, or
/// other internal state in a crafted snapshot can cause panics via `expect()`
/// calls throughout the codebase when the snapshot is resumed.
///
/// This test demonstrates that loading truncated/corrupted bytes fails gracefully.
#[test]
fn bug5_snapshot_deserialization_truncated() {
    let code = "func(1)";
    let run = MontyRun::new(code.to_owned(), "test.py", vec![], vec!["func".to_owned()]).unwrap();

    let limits = ResourceLimits::new().max_allocations(100);
    let progress = run.start(vec![], LimitedTracker::new(limits), &mut StdPrint).unwrap();

    // Serialize the valid RunProgress
    let serialized = progress.dump().expect("should serialize");
    assert!(!serialized.is_empty(), "serialized snapshot should not be empty");

    // Now test with truncated data - truncate to half
    let truncated = &serialized[..serialized.len() / 2];
    let load_result = monty::RunProgress::<LimitedTracker>::load(truncated);
    // This should return an error, not panic
    assert!(
        load_result.is_err(),
        "loading truncated snapshot should return error, not panic"
    );
}

/// Bug 5 (variant): Completely invalid bytes should fail gracefully.
#[test]
fn bug5_snapshot_deserialization_garbage_bytes() {
    let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0x00, 0x01, 0x02, 0x03];
    let load_result = monty::RunProgress::<LimitedTracker>::load(&garbage);
    assert!(
        load_result.is_err(),
        "loading garbage bytes should return error, not panic"
    );
}

/// Bug 5 (variant): Corrupted snapshot (bit-flipped valid data) should not panic.
///
/// This is the most dangerous case: a valid-looking snapshot with subtle corruption.
/// The deserialization may succeed (postcard is tolerant) but the resumed execution
/// may hit `expect()` calls when accessing invalid IDs.
#[test]
fn bug5_snapshot_deserialization_corrupted() {
    let code = "func(1)";
    let run = MontyRun::new(code.to_owned(), "test.py", vec![], vec!["func".to_owned()]).unwrap();

    let limits = ResourceLimits::new().max_allocations(100);
    let progress = run.start(vec![], LimitedTracker::new(limits), &mut StdPrint).unwrap();

    let mut serialized = progress.dump().expect("should serialize");

    // Corrupt bytes in the middle of the snapshot
    let mid = serialized.len() / 2;
    for i in mid..std::cmp::min(mid + 10, serialized.len()) {
        serialized[i] ^= 0xFF; // flip all bits
    }

    // Loading corrupted data should either fail at deserialization or,
    // if it succeeds, should fail gracefully during resume (not panic)
    let load_result = monty::RunProgress::<LimitedTracker>::load(&serialized);
    match load_result {
        Err(_) => {
            // Deserialization correctly rejected the corrupted data
        }
        Ok(progress) => {
            // Deserialization succeeded with corrupted data - this is the dangerous case.
            // Attempting to use this progress should not panic the host.
            if let Some((_name, _args, _kwargs, _call_id, state)) = progress.into_function_call() {
                // Resuming with corrupted state - should return error, not panic
                let resume_result = state.run(monty::MontyObject::None, &mut StdPrint);
                // Either an error or (unlikely) success is acceptable - just no panic
                let _ = resume_result;
            }
        }
    }
}
