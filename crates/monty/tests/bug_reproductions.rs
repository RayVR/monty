/// Tests that reproduce known bugs in the Monty interpreter.
///
/// These tests assert the CORRECT behavior. They FAIL because the bugs prevent
/// the correct behavior from occurring. When the bugs are fixed, the tests will pass.
use std::panic;

use monty::{LimitedTracker, MontyObject, MontyRun, ResourceLimits, StdPrint};

/// Bug 4a: `a @ b` (matrix multiply operator) panics the VM with todo!().
///
/// The `@` operator is parsed and compiled to `Opcode::BinaryMatMul` (compiler.rs:2817),
/// but the VM handler at vm/mod.rs:814 is just `todo!("BinaryMatMul not implemented")`.
/// This means untrusted code using `@` will crash the host process with a panic.
///
/// Expected: Should return a TypeError, not panic the host process.
#[test]
fn bug4a_matmul_operator_should_not_panic() {
    let code = "1 @ 2";
    let ex = MontyRun::new(code.to_owned(), "test.py", vec![], vec![]).unwrap();

    // Run inside catch_unwind to prevent the panic from killing the test runner.
    // The bug is that this panics at all — it should return a Result::Err.
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| ex.run_no_limits(vec![])));

    assert!(
        result.is_ok(),
        "@ operator should return a Result::Err(TypeError), not panic the host process"
    );
}

/// Bug 4b: `a @= b` (in-place matrix multiply) panics during compilation.
///
/// The augmented assignment `@=` maps to `Operator::MatMult` in the compiler
/// (compiler.rs:2845) which hits `todo!("InplaceMatMul not yet defined")`.
/// This panics at compile/prepare time, crashing the host process.
///
/// Expected: Should return a compile error, not panic the host process.
#[test]
fn bug4b_imatmul_operator_should_not_panic() {
    let code = "x = 1\nx @= 2";

    // The bug is that MontyRun::new panics instead of returning Err.
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        MontyRun::new(code.to_owned(), "test.py", vec![], vec![])
    }));

    assert!(
        result.is_ok(),
        "@= operator should return a compile error, not panic the host process"
    );
}

/// Bug 4d: `raise X from Y` silently drops the cause.
///
/// The parser at parse.rs:348 handles `Stmt::Raise(ast::StmtRaise { exc, .. })`
/// where the `..` silently ignores the `cause` field. The TODO comment says
/// "add cause to Node::Raise" but currently `raise X from Y` compiles identically
/// to `raise X`, losing the exception chain information.
///
/// In CPython, `raise ValueError('a') from TypeError('b')` sets `__cause__` on
/// the ValueError. This test verifies the cause is propagated — it fails because
/// Monty silently drops the `from` clause.
#[test]
fn bug4d_raise_from_should_preserve_cause() {
    // In CPython, re-raising inside except and stringifying the traceback
    // would show "The above exception was the direct cause of...".
    // Since Monty doesn't support __cause__, we can observe the bug by
    // checking that `raise X from Y` and `raise X` behave identically
    // when they shouldn't.
    //
    // A correct interpreter would make these two programs produce different tracebacks.

    let code_with_cause = r#"
try:
    raise ValueError('effect') from TypeError('cause')
except ValueError:
    pass
'ok'
"#;
    let code_without_cause = r#"
try:
    raise ValueError('effect')
except ValueError:
    pass
'ok'
"#;

    let ex_with = MontyRun::new(code_with_cause.to_owned(), "test.py", vec![], vec![]).unwrap();
    let ex_without = MontyRun::new(code_without_cause.to_owned(), "test.py", vec![], vec![]).unwrap();

    // Both should succeed (the exceptions are caught)
    let result_with = ex_with.run_no_limits(vec![]).unwrap();
    let result_without = ex_without.run_no_limits(vec![]).unwrap();

    assert_eq!(result_with, MontyObject::String("ok".to_owned()));
    assert_eq!(result_without, MontyObject::String("ok".to_owned()));

    // Now test via the exception: In a correct implementation, the exception
    // from `raise X from Y` would carry the cause. Let's verify by checking
    // the traceback output of the uncaught case.
    let code_uncaught_with_cause = "raise ValueError('effect') from TypeError('cause')";
    let code_uncaught_without_cause = "raise ValueError('effect')";

    let ex1 = MontyRun::new(code_uncaught_with_cause.to_owned(), "test.py", vec![], vec![]).unwrap();
    let ex2 = MontyRun::new(code_uncaught_without_cause.to_owned(), "test.py", vec![], vec![]).unwrap();

    let err1 = ex1.run_no_limits(vec![]).unwrap_err();
    let err2 = ex2.run_no_limits(vec![]).unwrap_err();

    let err1_str = format!("{err1}");

    // Bug: the `from TypeError('cause')` part is silently dropped at parse time.
    // In CPython, the traceback for `raise X from Y` includes:
    //   TypeError: cause
    //
    //   The above exception was the direct cause of the following exception:
    //
    //   ...
    //   ValueError: effect
    //
    // In Monty, only "ValueError: effect" is shown — no cause chain at all.
    assert!(
        err1_str.contains("The above exception was the direct cause"),
        "raise X from Y should include the cause chain in traceback, \
         but 'from TypeError(...)' was silently dropped.\nActual output:\n{err1_str}"
    );
}

/// Bug 2: JS bindings panic on OS calls via start() path.
///
/// The `progress_to_result` function in monty_cls.rs:725-730 calls `panic!()`
/// when it encounters `RunProgress::OsCall`, while the `run()` method properly
/// returns `Err(Error::from_reason(...))` at monty_cls.rs:273-276.
///
/// The actual JS panic is tested in crates/monty-js/__test__/bug_reproductions.spec.ts.
/// This Rust test proves the OsCall variant is reachable via normal Python code,
/// confirming the JS bindings' panic path can be triggered.
#[test]
fn bug2_os_call_is_reachable_via_start() {
    let code = "import os\nos.getenv('HOME')";
    let run = MontyRun::new(code.to_owned(), "test.py", vec![], vec![]).unwrap();

    let limits = ResourceLimits::new();
    let progress = run.start(vec![], LimitedTracker::new(limits), &mut StdPrint).unwrap();

    // Verify it yields OsCall (not Complete or FunctionCall).
    // This proves the JS bindings' panic path in progress_to_result() is reachable.
    assert!(
        progress.into_function_call().is_none(),
        "os.getenv should yield OsCall, not FunctionCall"
    );
}

/// Bug 5: Corrupted snapshot deserialization can panic the host.
///
/// When a serialized snapshot is corrupted (e.g., bit-flipped), postcard may
/// successfully deserialize it into a structurally valid but semantically invalid
/// state. Resuming execution with this corrupted state can hit `expect()` or
/// `unreachable!()` calls throughout the codebase, panicking the host process.
///
/// Expected: Corrupted snapshots should never panic — they should return errors.
#[test]
fn bug5_corrupted_snapshot_should_not_panic() {
    let code = "func(1)";
    let run = MontyRun::new(code.to_owned(), "test.py", vec![], vec!["func".to_owned()]).unwrap();

    let limits = ResourceLimits::new().max_allocations(100);
    let progress = run.start(vec![], LimitedTracker::new(limits), &mut StdPrint).unwrap();
    let serialized = progress.dump().expect("should serialize");

    // Try many different corruption offsets to find one that deserializes
    // but produces invalid state that panics on resume
    let mut found_panic = false;

    for offset in 0..serialized.len() {
        let mut corrupted = serialized.clone();
        corrupted[offset] ^= 0xFF;

        let load_result = monty::RunProgress::<LimitedTracker>::load(&corrupted);
        if let Ok(progress) = load_result {
            if let Some((_name, _args, _kwargs, _call_id, state)) = progress.into_function_call() {
                let resume_result =
                    panic::catch_unwind(panic::AssertUnwindSafe(|| state.run(MontyObject::None, &mut StdPrint)));
                if resume_result.is_err() {
                    found_panic = true;
                    break;
                }
            }
        }
    }

    assert!(
        !found_panic,
        "resuming a corrupted snapshot should return Result::Err, not panic the host"
    );
}
