use std::borrow::Cow;
use std::collections::HashMap;
use std::iter::Peekable;
use std::slice::Iter;

use data_structures::{BlockStack, DeclarationRequest, ExprStack, VarTracker};
use exalt_ast::{Notation, Operator, Precedence};
use exalt_lir::{CallbackArg, Function, Game, Opcode, RawScript};

mod data_structures;
mod ir;
mod refining;
mod transform;

use anyhow::{anyhow, bail, Result};
use ir::{Annotation, Case, Decl, Expr, FrameId, Literal, Reference, Script, Stmt};

use itertools::Itertools;
pub use transform::IrTransform;

#[derive(Clone, Copy)]
enum AssignState {
    Normal,
    Shorthand,
}

pub struct DecompilerState<'a> {
    game: Game,
    opcodes: Peekable<Iter<'a, Opcode>>,
    functions: &'a HashMap<usize, (String, usize)>,
    expr_stack: ExprStack<'a>,
    block_stack: BlockStack<'a>,
    assign_state: AssignState,
}

impl<'a> DecompilerState<'a> {
    pub fn new(
        game: Game,
        opcodes: Peekable<Iter<'a, Opcode>>,
        functions: &'a HashMap<usize, (String, usize)>,
    ) -> Self {
        Self {
            game,
            opcodes,
            functions,
            expr_stack: ExprStack::default(),
            block_stack: BlockStack::default(),
            assign_state: AssignState::Normal,
        }
    }
}

pub fn decompile(
    script: &RawScript,
    transform: Option<IrTransform>,
    game: Game,
    debug: bool,
) -> Result<String> {
    let ir_transform = transform.unwrap_or_default();
    let mut functions = HashMap::new();
    let mut global_var_tracker = VarTracker::new(script.global_frame_size);
    for (i, func) in script.functions.iter().enumerate() {
        if func.event == 0 {
            functions.insert(
                i,
                (
                    func.name.clone().unwrap_or_else(|| format!("anonfn{}", i)),
                    func.arity as usize,
                ),
            );
        }
    }
    let mut decls = Vec::new();
    for (i, func) in script.functions.iter().enumerate() {
        decls.push(decompile_function(
            game,
            &mut global_var_tracker,
            &functions,
            func,
            i,
            debug,
        )?);
    }
    let mut script = Script(decls);
    global_var_tracker.find_empty_array_inits()?;
    let extra_declarations = global_var_tracker.build_declaration_requests();
    refining::inject_global_var_declarations(&mut script, &extra_declarations);
    ir::pretty_print(&script, &ir_transform)
}

fn decompile_function<'a>(
    game: Game,
    global_var_tracker: &mut VarTracker,
    functions: &'a HashMap<usize, (String, usize)>,
    function: &'a Function,
    id: usize,
    debug: bool,
) -> Result<Decl<'a>> {
    let mut state = DecompilerState::new(game, function.code.iter().peekable(), functions);
    state.block_stack.push();
    while state.opcodes.peek().is_some() {
        decompile_opcode(&mut state)?;
    }
    let mut body = state.block_stack.pop()?;
    let has_default_return = refining::strip_default_return(&mut body);
    let mut block = Stmt::Block(body);
    // Collapse loops
    refining::collapse_for_loops(&mut block);
    refining::collapse_while_loops(&mut block);
    refining::add_match_breaks(&mut block);
    // Loops use jumps/labels which are no longer required, so get rid of them
    refining::prune_unused_labels(&mut block);
    // Analyze variable usage to identify arrays. Specifically looking for:
    // - Static array initializations. Shows up as a chain of assignments where only the first var is used/indexed.
    // - Empty array initializations. Shows up as assign/index for the first var and no direct references to the other indices.
    let param_count = if function.event != 0 {
        0
    } else {
        function.arity.into()
    };
    let mut var_info = refining::collect_var_details(
        &block,
        param_count,
        function.frame_size,
        global_var_tracker,
    )?;
    // TODO: Make it possible to reassign arrays in the compiler so we can take this step out
    if !debug {
        refining::collapse_static_array_inits(&mut block, &mut var_info)?;
    }
    var_info.find_empty_array_inits()?;
    let local_var_declarations = if debug {
        // In debug mode, we declare every variable at the top of the function.
        // This is useful for debugging/testing because it makes sure every variable
        // gets an identical frame index during a round trip.
        var_info.build_declaration_requests()
    } else {
        // Under normal conditions, var declarations are noisy and don't add anything to the source.
        // Variables may receive different frame indices during compiling, but the resulting
        // script is functionally identical.
        var_info
            .build_declaration_requests()
            .into_iter()
            .filter(|r| !matches!(r, DeclarationRequest::Var(_)))
            .collect_vec()
    };
    refining::inject_var_declarations(&mut block, &local_var_declarations);
    // TODO: Inject global vars at the top level

    let mut decl = if function.event == 0 {
        let name = function
            .name
            .clone()
            .unwrap_or_else(|| format!("anonfn{}", id));
        Decl::Function(Vec::new(), name, function.arity.into(), block)
    } else {
        let mut args = Vec::new();
        for arg in &function.args {
            args.push(match arg {
                CallbackArg::Int(v) => Literal::Int(*v),
                CallbackArg::Str(v) => Literal::Str(Cow::Borrowed(v)),
                CallbackArg::Float(v) => Literal::Float(*v),
            });
        }
        Decl::Callback(Vec::new(), function.event, args, block)
    };
    if !function.prefix.is_empty() {
        decl.append_annotation(Annotation::Prefix(&function.prefix));
    }
    if !function.suffix.is_empty() {
        decl.append_annotation(Annotation::Suffix(&function.suffix));
    }
    if function.unknown != 0 {
        decl.append_annotation(Annotation::Unknown(function.unknown));
    }
    if !has_default_return {
        decl.append_annotation(Annotation::NoDefaultReturn);
    }
    Ok(decl)
}

fn decompile_until(state: &mut DecompilerState, label: &str) -> Result<()> {
    while let Some(opcode) = state.opcodes.peek() {
        if let Opcode::Label(current_label) = opcode {
            if label == current_label {
                break;
            }
        }
        decompile_opcode(state)?;
    }
    Ok(())
}

fn decompile_opcode(state: &mut DecompilerState) -> Result<()> {
    let opcode = if let Some(opcode) = state.opcodes.next() {
        opcode
    } else {
        return Ok(());
    };
    match opcode {
        Opcode::Done => {}
        Opcode::VarLoad(id) => {
            let frame_id = FrameId(*id as usize, false);
            state.expr_stack.push(Expr::Ref(Reference::Var(frame_id)));
        }
        Opcode::ArrLoad(id) => {
            let index = Box::new(state.expr_stack.pop()?);
            let frame_id = FrameId(*id as usize, false);
            state
                .expr_stack
                .push(Expr::Ref(Reference::Index(frame_id, index)));
        }
        Opcode::PtrLoad(id) => {
            let index = Box::new(state.expr_stack.pop()?);
            let frame_id = FrameId(*id as usize, false);
            state
                .expr_stack
                .push(Expr::Ref(Reference::Dereference(frame_id, index)));
        }
        Opcode::VarAddr(id) => {
            let frame_id = FrameId(*id as usize, false);
            state.expr_stack.push(Expr::Addr(Reference::Var(frame_id)));
        }
        Opcode::ArrAddr(id) => {
            let index = Box::new(state.expr_stack.pop()?);
            let frame_id = FrameId(*id as usize, false);
            state
                .expr_stack
                .push(Expr::Addr(Reference::Index(frame_id, index)));
        }
        Opcode::PtrAddr(id) => {
            let index = Box::new(state.expr_stack.pop()?);
            let frame_id = FrameId(*id as usize, false);
            state
                .expr_stack
                .push(Expr::Addr(Reference::Dereference(frame_id, index)));
        }
        Opcode::GlobalVarLoad(id) => {
            let frame_id = FrameId(*id as usize, true);
            state.expr_stack.push(Expr::Ref(Reference::Var(frame_id)));
        }
        Opcode::GlobalArrLoad(id) => {
            let index = Box::new(state.expr_stack.pop()?);
            let frame_id = FrameId(*id as usize, true);
            state
                .expr_stack
                .push(Expr::Ref(Reference::Index(frame_id, index)));
        }
        Opcode::GlobalPtrLoad(id) => {
            let index = Box::new(state.expr_stack.pop()?);
            let frame_id = FrameId(*id as usize, true);
            state
                .expr_stack
                .push(Expr::Ref(Reference::Dereference(frame_id, index)));
        }
        Opcode::GlobalVarAddr(id) => {
            let frame_id = FrameId(*id as usize, true);
            state.expr_stack.push(Expr::Addr(Reference::Var(frame_id)));
        }
        Opcode::GlobalArrAddr(id) => {
            let index = Box::new(state.expr_stack.pop()?);
            let frame_id = FrameId(*id as usize, true);
            state
                .expr_stack
                .push(Expr::Addr(Reference::Index(frame_id, index)));
        }
        Opcode::GlobalPtrAddr(id) => {
            let index = Box::new(state.expr_stack.pop()?);
            let frame_id = FrameId(*id as usize, true);
            state
                .expr_stack
                .push(Expr::Addr(Reference::Dereference(frame_id, index)));
        }
        Opcode::IntLoad(v) => state.expr_stack.push(Expr::Literal(Literal::Int(*v))),
        Opcode::StrLoad(v) => state
            .expr_stack
            .push(Expr::Literal(Literal::Str(Cow::Borrowed(v)))),
        Opcode::FloatLoad(v) => state.expr_stack.push(Expr::Literal(Literal::Float(*v))),
        Opcode::Dereference => state.assign_state = AssignState::Shorthand,
        Opcode::Consume => {
            let expr = state.expr_stack.pop()?;
            state.block_stack.line(Stmt::Expr(expr))?;
        }
        Opcode::CompleteAssign | Opcode::Assign => decompile_assignment(state)?,
        Opcode::Fix => {
            let args = vec![state.expr_stack.pop()?];
            state
                .expr_stack
                .push(Expr::Call(Cow::Borrowed("int"), args))
        }
        Opcode::Float => {
            let args = vec![state.expr_stack.pop()?];
            state
                .expr_stack
                .push(Expr::Call(Cow::Borrowed("float"), args))
        }
        Opcode::Add => decompile_binary_expr(state, Operator::Add)?,
        Opcode::FloatAdd => decompile_binary_expr(state, Operator::FloatAdd)?,
        Opcode::Subtract => decompile_binary_expr(state, Operator::Subtract)?,
        Opcode::FloatSubtract => decompile_binary_expr(state, Operator::FloatSubtract)?,
        Opcode::Multiply => decompile_binary_expr(state, Operator::Multiply)?,
        Opcode::FloatMultiply => decompile_binary_expr(state, Operator::FloatMultiply)?,
        Opcode::Divide => decompile_binary_expr(state, Operator::Divide)?,
        Opcode::FloatDivide => decompile_binary_expr(state, Operator::FloatDivide)?,
        Opcode::Modulo => decompile_binary_expr(state, Operator::Modulo)?,
        Opcode::IntNegate => decompile_unary_expr(state, Operator::Negate)?,
        Opcode::FloatNegate => decompile_unary_expr(state, Operator::FloatNegate)?,
        Opcode::BinaryNot => decompile_unary_expr(state, Operator::BitwiseNot)?,
        Opcode::LogicalNot => decompile_unary_expr(state, Operator::LogicalNot)?,
        Opcode::BinaryOr => decompile_binary_expr(state, Operator::BitwiseOr)?,
        Opcode::BinaryAnd => decompile_binary_expr(state, Operator::BitwiseAnd)?,
        Opcode::Xor => decompile_binary_expr(state, Operator::Xor)?,
        Opcode::LeftShift => decompile_binary_expr(state, Operator::LeftShift)?,
        Opcode::RightShift => decompile_binary_expr(state, Operator::RightShift)?,
        Opcode::Equal => decompile_binary_expr(state, Operator::Equal)?,
        Opcode::FloatEqual => decompile_binary_expr(state, Operator::FloatEqual)?,
        Opcode::Exlcall => todo!(),
        Opcode::NotEqual => decompile_binary_expr(state, Operator::NotEqual)?,
        Opcode::FloatNotEqual => decompile_binary_expr(state, Operator::FloatNotEqual)?,
        Opcode::Nop0x3D => {}
        Opcode::LessThan => decompile_binary_expr(state, Operator::LessThan)?,
        Opcode::FloatLessThan => decompile_binary_expr(state, Operator::FloatLessThan)?,
        Opcode::LessThanEqualTo => decompile_binary_expr(state, Operator::LessThanEqualTo)?,
        Opcode::FloatLessThanEqualTo => {
            decompile_binary_expr(state, Operator::FloatLessThanEqualTo)?
        }
        Opcode::GreaterThan => decompile_binary_expr(state, Operator::GreaterThan)?,
        Opcode::FloatGreaterThan => decompile_binary_expr(state, Operator::FloatGreaterThan)?,
        Opcode::GreaterThanEqualTo => decompile_binary_expr(state, Operator::GreaterThanEqualTo)?,
        Opcode::FloatGreaterThanEqualTo => {
            decompile_binary_expr(state, Operator::FloatGreaterThanEqualTo)?
        }
        Opcode::CallById(id) => {
            let (name, arity) = state
                .functions
                .get(id)
                .ok_or_else(|| anyhow!("bad function id {}", id))?;
            let args = state.expr_stack.pop_args((*arity) as usize)?;
            state.expr_stack.push(Expr::Call(Cow::Borrowed(name), args));
        }
        Opcode::CallByName(name, arity) => {
            let args = state.expr_stack.pop_args((*arity) as usize)?;
            state.expr_stack.push(Expr::Call(Cow::Borrowed(name), args))
        }
        Opcode::Return => {
            let value = state.expr_stack.pop()?;
            state.block_stack.line(Stmt::Return(Some(value)))?;
        }
        Opcode::Jump(label) => state.block_stack.line(Stmt::Goto(label))?,
        Opcode::JumpNotZero(_) => bail!("found opcode for match stmt outside of match context"),
        Opcode::Or(label) => {
            decompile_short_circuited_binary_expr(state, label, Operator::LogicalOr)?
        }
        Opcode::JumpZero(label) => decompile_if(state, label)?,
        Opcode::And(label) => {
            decompile_short_circuited_binary_expr(state, label, Operator::LogicalAnd)?
        }
        Opcode::Yield => state.block_stack.line(Stmt::Yield)?,
        Opcode::Format(arity) => {
            let args = state.expr_stack.pop_args((*arity) as usize)?;
            state.block_stack.line(Stmt::Printf(args))?;
        }
        Opcode::Inc => decompile_inc(state, Operator::Increment)?,
        Opcode::Dec => decompile_inc(state, Operator::Decrement)?,
        Opcode::Copy => decompile_match(state)?,
        Opcode::ReturnFalse => state
            .block_stack
            .line(Stmt::Return(Some(Expr::Literal(Literal::Int(0)))))?,
        Opcode::ReturnTrue => state
            .block_stack
            .line(Stmt::Return(Some(Expr::Literal(Literal::Int(1)))))?,
        Opcode::Label(label) => state.block_stack.line(Stmt::Label(label))?,
        Opcode::StringEquals => {
            let args = state.expr_stack.pop_args(2)?;
            state
                .expr_stack
                .push(Expr::Call(Cow::Borrowed("streq"), args))
        }
        Opcode::StringNotEquals => {
            let args = state.expr_stack.pop_args(2)?;
            state
                .expr_stack
                .push(Expr::Call(Cow::Borrowed("strne"), args))
        }
        Opcode::Nop0x40 => {}
    }
    Ok(())
}

fn decompile_assignment(state: &mut DecompilerState) -> Result<()> {
    if let AssignState::Normal = state.assign_state {
        let right = state.expr_stack.pop()?;
        let left = state.expr_stack.pop()?;
        if let Expr::Addr(left) = left {
            state
                .block_stack
                .line(Stmt::Assign(Operator::Assign, left, right))?;
        } else {
            bail!("malformed assignment - left hand side is not a variable address");
        }
    } else if let Expr::Binary(op, left, right) = state.expr_stack.pop()? {
        let op = op
            .to_shorthand()
            .ok_or_else(|| anyhow!("malformed shorthand assignment - bad operator"))?;
        if let Expr::Addr(left) = *left {
            state.block_stack.line(Stmt::Assign(op, left, *right))?;
        } else {
            bail!("malformed assignment - left hand side is not a variable address");
        }
    } else {
        bail!("malformed shorthand assignment - top of stack was not a binary expression");
    }
    // In FE9, assignments are expressions. This is the only game where this is the case.
    // It isn't actually used, but it leads to inconsistencies in a couple places which we deal with here.
    if let Game::FE9 = state.game {
        if let Some(Opcode::Consume) = state.opcodes.peek() {
            state.opcodes.next();
        }
    }
    state.assign_state = AssignState::Normal;
    Ok(())
}

fn decompile_inc(state: &mut DecompilerState, op: Operator) -> Result<()> {
    let operand = state.expr_stack.pop()?;
    let notation = get_inc_notation(state, &operand);
    let operand = if let Expr::Addr(r) = operand {
        r
    } else {
        bail!("malformed increment - operand must be a variable address")
    };
    if let Notation::Prefix = notation {
        // Prefix inc leaves the new value on the stack afterwards
        // Read ahead until its on the stack for the next step
        consume_prefix_inc_value(state, &operand)?;
    }
    // Discard the old/new value since it's not relevant for decompiling
    state.expr_stack.pop()?;
    state.expr_stack.push(Expr::Inc(op, notation, operand));
    Ok(())
}

fn consume_prefix_inc_value(state: &mut DecompilerState, target: &Reference) -> Result<()> {
    while state.opcodes.peek().is_some() {
        decompile_opcode(state)?;
        if let Some(Expr::Ref(r)) = state.expr_stack.top() {
            if r == target {
                break;
            }
        }
    }
    Ok(())
}

fn get_inc_notation(state: &DecompilerState, operand: &Expr) -> Notation {
    // Must be a prefix inc if this isn't true - postfix inc leaves the old value on the stack first
    if let Some(top) = state.expr_stack.top() {
        if let (Expr::Ref(l), Expr::Addr(r)) = (top, operand) {
            if l.frame_id() == r.frame_id() {
                return Notation::Postfix;
            }
        }
    }
    Notation::Prefix
}

fn decompile_short_circuited_binary_expr(
    state: &mut DecompilerState,
    label: &str,
    op: Operator,
) -> Result<()> {
    let left = preserve_precedence(state.expr_stack.pop()?, op);
    decompile_until(state, label)?;
    let right = preserve_precedence(state.expr_stack.pop()?, op);
    state
        .expr_stack
        .push(Expr::Binary(op, Box::new(left), Box::new(right)));
    Ok(())
}

fn decompile_binary_expr(state: &mut DecompilerState, op: Operator) -> Result<()> {
    let right = preserve_precedence(state.expr_stack.pop()?, op);
    let left = preserve_precedence(state.expr_stack.pop()?, op);
    state
        .expr_stack
        .push(Expr::Binary(op, Box::new(left), Box::new(right)));
    Ok(())
}

fn decompile_unary_expr(state: &mut DecompilerState, op: Operator) -> Result<()> {
    let operand = preserve_precedence(state.expr_stack.pop()?, op);
    if let (Operator::Negate, Expr::Literal(Literal::Int(_))) = (op, &operand) {
        state
            .expr_stack
            .push(Expr::Call(Cow::Borrowed("negate"), vec![operand]));
    } else {
        state.expr_stack.push(Expr::Unary(op, Box::new(operand)));
    }
    Ok(())
}

fn preserve_precedence(operand: Expr, op: Operator) -> Expr {
    match operand {
        Expr::Binary(other_op, _, _) => {
            let op_precedence: Precedence = op.into();
            let operand_precedence: Precedence = other_op.into();
            if operand_precedence > op_precedence {
                operand
            } else {
                Expr::Grouped(Box::new(operand))
            }
        }
        _ => operand,
    }
}

fn decompile_if(state: &mut DecompilerState, label: &str) -> Result<()> {
    let check = state.expr_stack.pop()?;
    state.block_stack.push();
    decompile_until(state, label)?;
    let terminating_label = if let Some(Opcode::Label(label)) = state.opcodes.peek() {
        label
    } else {
        bail!("expected label after if");
    };
    let then_part = Box::new(Stmt::Block(state.block_stack.pop()?));
    state
        .block_stack
        .line(Stmt::If(check, then_part, None, terminating_label))?;
    Ok(())
}

fn decompile_match(state: &mut DecompilerState) -> Result<()> {
    let switch = state.expr_stack.pop()?;
    let mut cases = Vec::new();
    let mut default = None;
    let mut done_label: Option<&str>;
    loop {
        // Read conditions for the current case
        let mut conditions = Vec::new();
        loop {
            // Each case performs an equality check against the switch expr
            // Push the switch expr on to the stack so we can build this equality
            state.expr_stack.push(switch.clone());
            while let Some(opcode) = state.opcodes.peek() {
                if let Opcode::JumpNotZero(_) = opcode {
                    state.opcodes.next(); // Discard the jump
                    break;
                }
                decompile_opcode(state)?;
            }
            if let Expr::Binary(_, _, right) = state.expr_stack.pop()? {
                conditions.push(*right);
            } else {
                bail!("malformed match - invalid case condition");
            }
            if let Some(Opcode::Copy) = state.opcodes.peek() {
                // Need to read another condition.
                state.opcodes.next();
            } else {
                break;
            }
        }
        // Read the case body
        let next_case_label = if let Some(Opcode::Jump(label)) = state.opcodes.next() {
            label
        } else {
            bail!("malformed match - no jump to next case stmt")
        };
        state.block_stack.push();
        decompile_until(state, next_case_label)?;
        state.opcodes.next(); // Discard the label
        let mut body = state.block_stack.pop()?;
        let end_label = if let Some(Stmt::Goto(label)) = body.pop() {
            label
        } else {
            bail!("malformed match - case did not end with a jump")
        };
        done_label = Some(end_label);
        cases.push(Case {
            conditions,
            body: Stmt::Block(body),
        });
        // Figure out where to go next
        match state.opcodes.peek() {
            Some(Opcode::Consume) => {
                // Done!
                state.opcodes.next();
                break;
            }
            Some(Opcode::Copy) => {
                // This is the start of another case
                state.opcodes.next();
            }
            Some(_) => {
                state.block_stack.push();
                decompile_until(state, end_label)?;
                let mut body = state.block_stack.pop()?;
                body.pop();
                state.opcodes.next(); // Consume the end label
                default = Some(Stmt::Block(body));
                state.opcodes.next(); // Consume the consume opcode
                break;
            }
            _ => bail!("unexpected end of match"),
        }
    }
    state.block_stack.line(Stmt::Match(
        switch,
        cases,
        default.map(Box::new),
        done_label.ok_or_else(|| anyhow!("could not find match done label"))?,
    ))?;
    Ok(())
}
