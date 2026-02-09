# Monty Codebase Review

Comprehensive review of bugs, limitations, and potential enhancements.

---

## Critical Bugs

### 1. Reference Count Leak in `Heap::mult_sequence()` on Allocation Failure

**File:** `crates/monty/src/heap.rs:1388-1465`

When multiplying a list or tuple by a count, reference counts for contained `Ref` values are incremented *before* the new container is allocated. If `allocate()` then fails due to resource limits, the error propagates via `?` but the incremented reference counts are never decremented. The items vector is also forgotten via `std::mem::forget(items)`, so there is no path that cleans up.

```rust
// Lines 1403-1407: refcounts incremented
for ref_id in &ref_ids {
    for _ in 0..count {
        self.inc_ref(*ref_id);
    }
}
// ...
std::mem::forget(items);  // Line 1422: items forgotten
// Line 1424: allocate() can fail here, leaking all the incremented refcounts
Ok(Some(Value::Ref(self.allocate(HeapData::List(List::new(result)))?)))
```

**Fix:** Either check resource limits before incrementing refcounts, or decrement on the error path.

### 2. Panics in JS Bindings on Async/OS Calls Crash Node.js Process

**File:** `crates/monty-js/src/monty_cls.rs:725-730`

```rust
RunProgress::ResolveFutures(_) => {
    panic!("Async futures (ResolveFutures) are not yet supported in the JS bindings")
}
RunProgress::OsCall { function, .. } => {
    panic!("OS calls are not yet supported in the JS bindings: {function:?}")
}
```

If Python code triggers async futures or OS calls in the JS bindings, the entire Node.js process crashes with `panic!()` instead of returning a proper error. This is both a correctness and DoS issue since untrusted Python code can trigger these paths.

**Fix:** Return `Err(Error::from_reason(...))` instead of panicking.

### 3. Panics in `prepare.rs` on Closure Variable Resolution Failure

**File:** `crates/monty/src/prepare.rs:1153`

```rust
panic!("free_var '{var_name}' not found in enclosing scope's cell_var_map or free_var_map");
```

If the scope analysis misidentifies a free variable, the interpreter panics instead of raising a proper `NameError`. While this should only occur due to compiler bugs, a panic is the wrong response — it crashes the host process.

**Fix:** Return a `RunError` or `ExcType::NameError` instead of panicking.

### 4. `todo!()` Panics in VM for Reachable Opcodes

Several opcodes panic with `todo!()` when executed. These are reachable by user code:

| Opcode | File:Line | Trigger |
|--------|-----------|---------|
| `DeleteSubscr` | `vm/mod.rs:1031` | `del obj[key]` |
| `DeleteAttr` | `vm/mod.rs:1049` | `del obj.attr` |
| `RaiseFrom` | `vm/mod.rs:1293` | `raise X from Y` |
| `BinaryMatMul` | `vm/mod.rs:814` | `a @ b` |

These will crash the host process if untrusted code uses these Python features. They should return proper errors ("not implemented" or equivalent `TypeError`) instead of panicking.

---

## Security Concerns

### 5. Snapshot Deserialization Lacks Validation

**File:** `crates/monty/src/run.rs`, `crates/monty-python/src/monty_cls.rs:832-853`

Serialized snapshots (`postcard::from_bytes()`) are loaded without validation. Invalid function IDs, heap IDs, or other internal state can cause panics via `expect()` calls throughout the codebase. While `postcard` is memory-safe (unlike `pickle`), this is a DoS vector if snapshots come from untrusted sources.

**Recommendation:** Add snapshot validation after deserialization, or document that snapshots must only come from trusted sources.

### 6. Array Size Overflow in JS Bindings

**File:** `crates/monty-js/src/convert.rs:166-169`

```rust
let mut arr = env.create_array(
    items.len().try_into().expect("array size overflows u32")
)?;
```

Uses `expect()` on `try_into()` for array sizes. If Monty produces a list with >2^32 elements, this panics. Should return a proper error.

---

## Limitations (Missing Python Features)

### Language Features Not Supported

These are fundamental Python features not yet implemented:

| Feature | Notes |
|---------|-------|
| **Classes/OOP** | No class definitions, no user-defined types |
| **Pattern matching** | `match`/`case` (Python 3.10+) |
| **Context managers** | `with` statement |
| **Generators** | `yield`, `yield from` |
| **`del` statement** | Delete variables/subscripts/attributes |
| **Exception chaining** | `raise X from Y` |
| **Async for/with** | Only basic async/await supported |
| **Complex numbers** | No complex literal or type |
| **Dict unpacking in literals** | `{**d}` |
| **Type aliases** | Python 3.12+ |

### Missing Built-in Functions (~44 of 69+)

Only ~25 built-in functions are implemented. Notable missing ones:

- `map`, `filter` — functional programming staples
- `int`, `float`, `str`, `list`, `dict`, `set`, `tuple` — as constructor functions
- `getattr`, `setattr`, `delattr`, `hasattr` — attribute access
- `callable`, `dir`, `vars`, `globals`, `locals` — introspection
- `eval`, `exec`, `compile` — dynamic execution (may be intentionally excluded for security)
- `iter` with sentinel — `iter(callable, sentinel)` explicitly returns error
- `format` — formatting function
- `open`, `input` — I/O (likely intentionally excluded)

### Missing String Methods

**File:** `crates/monty/src/types/str.rs:1482-1502` (documented in code)

- `str.format()` — complex format mini-language
- `str.format_map()`
- `str.maketrans()` / `str.translate()`
- `str.expandtabs()`
- `str.isprintable()`

### List Sort Limitation

**File:** `crates/monty/src/types/list.rs:954`

`list.sort(key=...)` only accepts built-in functions (like `len`, `abs`), not user-defined functions. User-defined key functions raise an error.

### Float-to-Int Precision

**File:** `crates/monty/src/types/type.rs`

`int(large_float)` clamps to `i64::MAX/MIN` instead of supporting arbitrary precision, diverging from CPython for large float values.

### Integer Range

Monty uses `i64` for integers instead of arbitrary precision. Very large integer arithmetic will overflow where CPython would succeed. A `LongInt` type exists (`types/long_int.rs`) as a partial mitigation.

---

## Potential Enhancements

### 7. Comprehension Stack Bounds Check

**File:** `crates/monty/src/bytecode/vm/collections.rs:314-323`

`ListAppend`, `SetAdd`, `DictSetItem` calculate `stack_len - 1 - depth` without bounds checking. If `depth >= stack_len`, this wraps around as unsigned arithmetic, causing a panic on the subsequent index access. An explicit bounds check would turn this into a proper error.

### 8. Dict `pop()` O(n) Performance

**File:** `crates/monty/src/types/dict.rs:1022`

`dict.pop()` has O(n) rebuild performance when removing the last entry. The code has a comment acknowledging this.

### 9. GC Interval Not Configurable

The garbage collection interval is hardcoded at 100,000 allocations. Making this configurable via resource limits could benefit different workloads.

### 10. Better Error Messages for Invalid Bytecode

Several internal error messages (e.g., "CallBuiltinFunction: invalid builtin_id") don't include the offending value, making debugging harder.

### 11. Opcode Sequence Validation

The VM doesn't validate opcode sequences at compile time or load time. Adding a bytecode verifier pass could catch compiler bugs early and prevent confusing runtime errors.

---

## Code Quality Observations

### Positive

- Excellent use of `defer_drop!` and `HeapGuard` for reference count safety
- Clean separation between sandbox and host via external function callbacks
- Module import system is properly restricted to built-in modules only
- Resource limits (time, memory, allocations, recursion) are well-enforced
- Minimal `unsafe` code (only 3 uses, all for `ManuallyDrop::take()`)
- Good use of Rust's type system to prevent misuse

### Areas for Improvement

- Several `panic!()` and `todo!()` calls in code paths reachable by user input should be converted to proper errors
- Some `expect()` calls in hot paths could be replaced with `?` for robustness
- The `mult_sequence` refcount-before-allocate pattern is fragile
