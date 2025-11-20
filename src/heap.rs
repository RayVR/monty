use crate::exceptions::Exception;
use crate::object::Object;

/// Unique identifier for objects stored inside the heap arena.
pub type ObjectId = usize;

/// Errors surfaced when interacting with the heap, primarily for invalid IDs.
#[allow(dead_code)]
#[derive(Debug)]
pub enum HeapError {
    InvalidId,
}

/// HeapData captures every runtime object that must live in the arena.
#[allow(dead_code)]
#[derive(Debug)]
pub enum HeapData {
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<Object>),
    Tuple(Vec<Object>),
    Exception(Exception),
}

/// A single entry inside the heap arena, storing refcount and payload.
#[derive(Debug)]
struct HeapObject {
    refcount: usize,
    data: HeapData,
}

/// Reference-counted arena that backs all heap-only runtime objects.
///
/// The heap never reuses IDs during a single execution; instead it appends new
/// entries and relies on `clear()` between runs.  This keeps identity checks
/// simple and avoids the need for generation counters while we're still
/// building out semantics.
#[derive(Debug)]
pub struct Heap {
    objects: Vec<Option<HeapObject>>,
}

impl Heap {
    /// Creates an empty heap ready to service allocations for a single executor run.
    pub fn new() -> Self {
        Self { objects: Vec::new() }
    }

    /// Allocates a new heap object, returning the fresh identifier.
    #[allow(dead_code)]
    pub fn allocate(&mut self, data: HeapData) -> ObjectId {
        let id = self.objects.len();
        self.objects.push(Some(HeapObject { refcount: 1, data }));
        id
    }

    /// Increments the reference count for an existing heap object.
    #[allow(dead_code)]
    pub fn inc_ref(&mut self, id: ObjectId) {
        if let Some(Some(object)) = self.objects.get_mut(id) {
            object.refcount += 1;
        }
    }

    /// Decrements the reference count and frees the object (plus children) once it hits zero.
    #[allow(dead_code)]
    pub fn dec_ref(&mut self, id: ObjectId) {
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            let Some(slot) = self.objects.get_mut(current) else {
                continue;
            };
            let Some(entry) = slot.as_mut() else {
                continue;
            };

            if entry.refcount > 1 {
                entry.refcount -= 1;
                continue;
            }

            let owned = slot.take().map(|owned| owned.data);
            if let Some(data) = owned {
                enqueue_children(&data, &mut stack);
            }
        }
    }

    /// Returns an immutable reference to the heap data stored at the given ID.
    #[allow(dead_code)]
    pub fn get(&self, id: ObjectId) -> Result<&HeapData, HeapError> {
        self.objects
            .get(id)
            .and_then(|slot| slot.as_ref())
            .map(|object| &object.data)
            .ok_or(HeapError::InvalidId)
    }

    /// Returns a mutable reference to the heap data stored at the given ID.
    #[allow(dead_code)]
    pub fn get_mut(&mut self, id: ObjectId) -> Result<&mut HeapData, HeapError> {
        self.objects
            .get_mut(id)
            .and_then(|slot| slot.as_mut())
            .map(|object| &mut object.data)
            .ok_or(HeapError::InvalidId)
    }

    /// Removes all objects and resets the ID counter, used between executor runs.
    pub fn clear(&mut self) {
        self.objects.clear();
    }
}

/// Pushes any child object IDs referenced by `data` onto the provided stack so
/// `dec_ref` can recursively drop entire object graphs without recursion.
#[allow(dead_code)]
fn enqueue_children(data: &HeapData, stack: &mut Vec<ObjectId>) {
    match data {
        HeapData::List(_items) | HeapData::Tuple(_items) => {
            // Non-heap references will be added in later phases; keep placeholders so the
            // match arms are ready once Object::Ref exists.
            let _ = stack;
        }
        HeapData::Exception(_exc) => {
            let _ = stack;
        }
        HeapData::Str(_) | HeapData::Bytes(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{Heap, HeapData};
    use crate::object::Object;

    #[test]
    fn allocate_and_get() {
        let mut heap = Heap::new();
        let id = heap.allocate(HeapData::Str("hello".to_string()));
        match heap.get(id).unwrap() {
            HeapData::Str(value) => assert_eq!(value, "hello"),
            _ => panic!("unexpected data"),
        }
    }

    #[test]
    fn refcount_behavior() {
        let mut heap = Heap::new();
        let list_id = heap.allocate(HeapData::List(vec![Object::Int(1), Object::Int(2)]));
        heap.inc_ref(list_id);
        heap.dec_ref(list_id);
        assert!(heap.get(list_id).is_ok());
        heap.dec_ref(list_id);
        assert!(heap.get(list_id).is_err());
    }
}
