mod evaluate;
mod exceptions;
mod expressions;
mod heap;
mod literal;
mod object;
mod object_types;
mod operators;
mod parse;
mod parse_error;
mod prepare;
mod run;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use crate::exceptions::{InternalRunError, RunError};
pub use crate::expressions::Exit;
use crate::expressions::Node;
use crate::heap::Heap;
pub use crate::object::Object;
use crate::parse::parse;
pub use crate::parse_error::{ParseError, ParseResult};
use crate::prepare::prepare;
use crate::run::RunFrame;
use std::cell::RefCell;

#[derive(Debug)]
pub struct Executor<'c> {
    initial_namespace: Vec<Object>,
    nodes: Vec<Node<'c>>,
    heap: RefCell<Heap>,
}

impl<'c> Executor<'c> {
    pub fn new(code: &'c str, filename: &'c str, input_names: &[&str]) -> ParseResult<'c, Self> {
        let nodes = parse(code, filename)?;
        // dbg!(&nodes);
        let (initial_namespace, nodes) = prepare(nodes, input_names)?;
        // dbg!(&initial_namespace, &nodes);
        Ok(Self {
            initial_namespace,
            nodes,
            heap: RefCell::new(Heap::new()),
        })
    }

    pub fn run(&self, inputs: Vec<Object>) -> Result<Exit<'c>, InternalRunError> {
        self.heap.borrow_mut().clear();
        let mut namespace = self.initial_namespace.clone();
        for (i, input) in inputs.into_iter().enumerate() {
            namespace[i] = input;
        }
        match RunFrame::new(namespace).execute(&self.nodes) {
            Ok(v) => Ok(v),
            Err(e) => match e {
                RunError::Exc(exc) => Ok(Exit::Raise(exc)),
                RunError::Internal(internal) => Err(internal),
            },
        }
    }
}

/// parse code and show the parsed AST, mostly for testing
pub fn parse_show(code: &str, filename: &str) -> Result<String, String> {
    match parse(code, filename) {
        Ok(ast) => Ok(format!("{ast:#?}")),
        Err(e) => Err(e.to_string()),
    }
}
