use std::borrow::Cow;

use crate::evaluate::{evaluate, evaluate_bool};
use crate::exceptions::{exc, exc_err, internal_err, ExcType, Exception, InternalRunError, RunError, StackFrame};
use crate::expressions::{Exit, ExprLoc, Identifier, Node};
use crate::object::Object;
use crate::operators::Operator;
use crate::parse::CodeRange;

pub type RunResult<'c, T> = Result<T, RunError<'c>>;

#[derive(Debug)]
pub(crate) struct RunFrame<'c> {
    namespace: Vec<Object>,
    parent: Option<StackFrame<'c>>,
    name: &'c str,
}

impl<'c> RunFrame<'c> {
    pub fn new(namespace: Vec<Object>) -> Self {
        Self {
            namespace,
            parent: None,
            name: "<module>",
        }
    }

    pub fn execute(&mut self, nodes: &[Node<'c>]) -> RunResult<'c, Exit<'c>> {
        for node in nodes {
            if let Some(leave) = self.execute_node(node)? {
                return Ok(leave);
            }
        }
        Ok(Exit::ReturnNone)
    }

    fn execute_node(&mut self, node: &Node<'c>) -> RunResult<'c, Option<Exit<'c>>> {
        match node {
            Node::Pass => return internal_err!(InternalRunError::Error; "Unexpected `pass` in execution"),
            Node::Expr(expr) => {
                self.execute_expr(expr)?;
            }
            Node::Return(expr) => return Ok(Some(Exit::Return(self.execute_expr(expr)?.into_owned()))),
            Node::ReturnNone => return Ok(Some(Exit::ReturnNone)),
            Node::Raise(exc) => self.raise(exc)?,
            Node::Assign { target, object } => {
                self.assign(target, object)?;
            }
            Node::OpAssign { target, op, object } => {
                self.op_assign(target, op, object)?;
            }
            Node::For {
                target,
                iter,
                body,
                or_else,
            } => self.for_loop(target, iter, body, or_else)?,
            Node::If { test, body, or_else } => self.if_(test, body, or_else)?,
        };
        Ok(None)
    }

    fn execute_expr<'d>(&'d mut self, expr: &'d ExprLoc<'c>) -> RunResult<'c, Cow<'d, Object>> {
        // it seems the struct creation is optimized away, and has no cost
        match evaluate(&mut self.namespace, expr) {
            Ok(object) => Ok(object),
            Err(mut e) => {
                set_name(self.name, &mut e);
                Err(e)
            }
        }
    }

    fn execute_expr_bool(&mut self, expr: &ExprLoc<'c>) -> RunResult<'c, bool> {
        match evaluate_bool(&mut self.namespace, expr) {
            Ok(object) => Ok(object),
            Err(mut e) => {
                set_name(self.name, &mut e);
                Err(e)
            }
        }
    }

    fn raise(&mut self, op_exc_expr: &Option<ExprLoc<'c>>) -> RunResult<'c, ()> {
        if let Some(exc_expr) = op_exc_expr {
            let object = self.execute_expr(exc_expr)?;
            let exc = match object.into_owned() {
                Object::Exc(exc) => exc,
                _ => return exc_err!(ExcType::TypeError; "exceptions must derive from BaseException"),
            };
            Err(exc.with_frame(self.stack_frame(&exc_expr.position)).into())
        } else {
            internal_err!(InternalRunError::TodoError; "plain raise not yet supported")
        }
    }

    fn assign(&mut self, target: &Identifier<'c>, expr: &ExprLoc<'c>) -> RunResult<'c, ()> {
        self.namespace[target.id] = self.execute_expr(expr)?.into_owned();
        Ok(())
    }

    fn op_assign(&mut self, target: &Identifier<'c>, op: &Operator, expr: &ExprLoc<'c>) -> RunResult<'c, ()> {
        // TODO ideally we wouldn't need to clone here since add_mut could take a cow
        let right_object = self.execute_expr(expr)?.into_owned();
        if let Some(target_object) = self.namespace.get_mut(target.id) {
            let r = match op {
                Operator::Add => target_object.add_mut(right_object),
                _ => return internal_err!(InternalRunError::TodoError; "Assign operator {op:?} not yet implemented"),
            };
            if let Err(right) = r {
                let target_type = target_object.to_string();
                let right_type = right.to_string();
                let e = exc!(ExcType::TypeError; "unsupported operand type(s) for {op}: '{target_type}' and '{right_type}'");
                Err(e.with_frame(self.stack_frame(&expr.position)).into())
            } else {
                Ok(())
            }
        } else {
            let e = Exception::new(target.name.to_string(), ExcType::NameError);
            Err(e.with_frame(self.stack_frame(&target.position)).into())
        }
    }

    fn for_loop(
        &mut self,
        target: &Identifier,
        iter: &ExprLoc<'c>,
        body: &[Node<'c>],
        _or_else: &[Node<'c>],
    ) -> RunResult<'c, ()> {
        let range_size = match self.execute_expr(iter)?.as_ref() {
            Object::Range(s) => *s,
            _ => return internal_err!(InternalRunError::TodoError; "`for` iter must be a range"),
        };

        for object in 0i64..range_size {
            self.namespace[target.id] = Object::Int(object);
            self.execute(body)?;
        }
        Ok(())
    }

    fn if_<'d>(&mut self, test: &'d ExprLoc<'c>, body: &'d [Node<'c>], or_else: &'d [Node<'c>]) -> RunResult<'c, ()> {
        if self.execute_expr_bool(test)? {
            self.execute(body)?;
        } else {
            self.execute(or_else)?;
        }
        Ok(())
    }

    fn stack_frame(&self, position: &CodeRange<'c>) -> StackFrame<'c> {
        StackFrame::new(position, self.name, &self.parent)
    }
}

fn set_name<'c>(name: &'c str, error: &mut RunError<'c>) {
    if let RunError::Exc(ref mut exc) = error {
        if let Some(ref mut stack_frame) = exc.frame {
            stack_frame.frame_name = Some(name);
        }
    }
}
