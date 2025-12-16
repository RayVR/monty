use monty::exceptions::{ExcType, SimpleException};

/// Tests that different exception types produce different hashes.
#[test]
fn exception_hash_different_types() {
    let value_error = SimpleException::new(ExcType::ValueError, Some("test".into()));
    let type_error = SimpleException::new(ExcType::TypeError, Some("test".into()));
    let key_error = SimpleException::new(ExcType::KeyError, Some("test".into()));

    // Different exception types should hash differently
    assert_ne!(value_error.py_hash(), type_error.py_hash());
    assert_ne!(value_error.py_hash(), key_error.py_hash());
    assert_ne!(type_error.py_hash(), key_error.py_hash());
}

/// Tests that different messages produce different hashes.
#[test]
fn exception_hash_different_messages() {
    let exc1 = SimpleException::new(ExcType::ValueError, Some("message one".into()));
    let exc2 = SimpleException::new(ExcType::ValueError, Some("message two".into()));
    let exc3 = SimpleException::new(ExcType::ValueError, None);

    // Different messages should hash differently
    assert_ne!(exc1.py_hash(), exc2.py_hash());
    assert_ne!(exc1.py_hash(), exc3.py_hash());
    assert_ne!(exc2.py_hash(), exc3.py_hash());
}

/// Tests that identical exceptions produce the same hash.
#[test]
fn exception_hash_same_exception() {
    let exc1 = SimpleException::new(ExcType::ValueError, Some("same message".into()));
    let exc2 = SimpleException::new(ExcType::ValueError, Some("same message".into()));

    // Same exception type and message should hash the same
    assert_eq!(exc1.py_hash(), exc2.py_hash());
}

/// Tests that exceptions with None argument hash consistently.
#[test]
fn exception_hash_none_arg() {
    let exc1 = SimpleException::new(ExcType::TypeError, None);
    let exc2 = SimpleException::new(ExcType::TypeError, None);

    // Same type with None args should hash the same
    assert_eq!(exc1.py_hash(), exc2.py_hash());

    // Different types with None args should hash differently
    let exc3 = SimpleException::new(ExcType::ValueError, None);
    assert_ne!(exc1.py_hash(), exc3.py_hash());
}

/// Tests that all exception types produce distinct hashes (no collisions for basic cases).
#[test]
fn exception_hash_all_types_distinct() {
    let exceptions = [
        SimpleException::new(ExcType::ValueError, None),
        SimpleException::new(ExcType::TypeError, None),
        SimpleException::new(ExcType::NameError, None),
        SimpleException::new(ExcType::AttributeError, None),
        SimpleException::new(ExcType::KeyError, None),
    ];

    // Collect all hashes
    let hashes: Vec<u64> = exceptions.iter().map(SimpleException::py_hash).collect();

    // Verify all hashes are unique
    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            assert_ne!(
                hashes[i], hashes[j],
                "Hash collision between exception types at indices {i} and {j}",
            );
        }
    }
}
