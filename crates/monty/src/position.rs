use crate::{evaluate::ExternalCall, value::Value};
use std::fmt::Debug;

/// Result of executing a frame - return, yield, or external function call.
///
/// When a frame encounters a `return` statement, it produces `Return(value)`.
/// When a frame encounters a `yield` statement, it produces `Yield(value)` to
/// pause execution and return control to the caller.
/// When a frame encounters a call to an external function, it produces
/// `FunctionCall` to pause execution and let the host provide the return value.
#[derive(Debug)]
pub enum FrameExit {
    /// Normal return from a function or end of module execution.
    Return(Value),
    /// External function call pauses execution.
    ///
    /// The host must provide the return value to resume execution. The arguments
    /// have already been evaluated and converted to `Value`.
    ExternalCall(ExternalCall),
}

pub trait AbstractPositionTracker: Clone + Debug {
    /// Get the next position to execute from
    fn next(&mut self) -> Position;

    /// When suspending execution, set the position to resume from
    fn record(&mut self, index: usize);

    /// When leaving an if statement or for loop, set the position to resume from
    fn set_clause_state(&mut self, clause_state: ClauseState);

    /// Whether to clear return values, this is only necessary when position is being tracked
    fn clear_return_values() -> bool;
}

#[derive(Debug, Clone)]
pub struct NoPositionTracker;

impl AbstractPositionTracker for NoPositionTracker {
    fn next(&mut self) -> Position {
        Position::default()
    }

    fn record(&mut self, _index: usize) {}

    fn set_clause_state(&mut self, _clause_state: ClauseState) {}

    fn clear_return_values() -> bool {
        false
    }
}

#[derive(Debug, Clone, Default)]
pub struct PositionTracker {
    pub stack: Vec<Position>,
    clause_state: Option<ClauseState>,
    incr: bool,
}

impl From<Vec<Position>> for PositionTracker {
    fn from(stack: Vec<Position>) -> Self {
        PositionTracker {
            stack,
            ..Default::default()
        }
    }
}

impl AbstractPositionTracker for PositionTracker {
    fn next(&mut self) -> Position {
        self.stack.pop().unwrap_or_default()
    }

    fn record(&mut self, index: usize) {
        let index = if self.incr {
            self.incr = false;
            index + 1
        } else {
            index
        };
        self.stack.push(Position {
            index,
            clause_state: self.clause_state.take(),
        });
    }

    fn set_clause_state(&mut self, clause_state: ClauseState) {
        self.clause_state = Some(clause_state);
    }

    fn clear_return_values() -> bool {
        true
    }
}

/// Represents a position within nested control flow for yield resumption.
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    /// Index of the next node to execute within the node array
    pub index: usize,
    /// indicates how to resume within the nested control flow if relevant
    pub clause_state: Option<ClauseState>,
}

#[derive(Debug, Clone, Copy)]
pub enum ClauseState {
    /// When resuming within the if statement,
    /// whether the condition was met - true to resume the if branch and false to resume the else branch
    If(bool),
    /// When resuming within a for loop,
    /// the index of the next element to iterate over
    For(usize),
}
