use std::collections::HashMap;

use exalt_ast::Operator;
use itertools::Itertools;

use crate::data_structures::{DeclarationRequest, VarTracker};
use crate::ir::{Expr, FrameId, Literal, Reference, Script, Stmt, Decl};
use anyhow::Result;

pub fn strip_default_return(block: &mut Vec<Stmt>) -> bool {
    if let Some(Stmt::Return(Some(Expr::Literal(Literal::Int(i))))) = block.last() {
        if *i == 0 {
            return block.pop().is_some();
        }
    }
    false
}

pub fn prune_unused_labels(stmt: &mut Stmt) {
    let mut counts = HashMap::new();
    count_label_references_recursive(stmt, &mut counts);
    prune_labels_recursive(stmt, &counts);
}

fn prune_labels_recursive(stmt: &mut Stmt, counts: &HashMap<String, usize>) {
    match stmt {
        Stmt::Block(contents) => {
            let mut i = 0;
            while i < contents.len() {
                if let Stmt::Label(label) = &contents[i] {
                    if *counts.get(*label).unwrap_or(&0) == 0 {
                        contents.remove(i);
                    } else {
                        i += 1;
                    }
                } else {
                    prune_labels_recursive(&mut contents[i], counts);
                    i += 1;
                }
            }
        }
        Stmt::For(_, _, _, body) => prune_labels_recursive(body, counts),
        Stmt::If(_, then_part, else_part, _) => {
            prune_labels_recursive(then_part, counts);
            if let Some(stmt) = else_part {
                prune_labels_recursive(stmt, counts);
            }
        }
        Stmt::Match(_, cases, default, _) => {
            for case in cases {
                prune_labels_recursive(&mut case.body, counts);
            }
            if let Some(stmt) = default {
                prune_labels_recursive(stmt, counts);
            }
        }
        Stmt::While(_, body) => prune_labels_recursive(body, counts),
        _ => {}
    }
}

fn count_label_references_recursive(stmt: &Stmt, counts: &mut HashMap<String, usize>) {
    match stmt {
        Stmt::Block(stmts) => {
            for stmt in stmts {
                count_label_references_recursive(stmt, counts);
            }
        }
        Stmt::For(_, _, _, body) => count_label_references_recursive(body, counts),
        Stmt::Goto(label) => {
            let count = counts.entry(label.to_string()).or_insert(0);
            *count += 1;
        }
        Stmt::If(_, then_part, else_part, _) => {
            count_label_references_recursive(then_part, counts);
            if let Some(stmt) = else_part {
                count_label_references_recursive(stmt, counts);
            }
        }
        Stmt::Match(_, cases, default, _) => {
            for case in cases {
                count_label_references_recursive(&case.body, counts);
            }
            if let Some(stmt) = default {
                count_label_references_recursive(stmt, counts);
            }
        }
        Stmt::While(_, body) => count_label_references_recursive(body, counts),
        _ => {}
    }
}

pub fn collapse_while_loops(stmt: &mut Stmt) {
    match stmt {
        Stmt::Block(contents) => {
            let mut i = 0;
            while i + 1 < contents.len() {
                if is_while_loop_sequence(&contents[i..i + 2]) {
                    // TODO: Revisit this at some point.
                    //       Blocks should be small so it's not the end of the world, but this is a lot of shifting.
                    let continue_label = contents.remove(i).unwrap_label();
                    let (check, mut body, _, break_label) = contents.remove(i).unwrap_if();
                    if let Stmt::Block(contents) = body.as_mut() {
                        contents.pop(); // Remove the jump to the loop check - this is implicit in the source
                    }
                    replace_jumps_with_break_and_continue(&mut body, break_label, continue_label);
                    collapse_while_loops(&mut body);
                    contents.insert(i, Stmt::While(check, body));
                }
                collapse_while_loops(&mut contents[i]);
                i += 1;
            }
            for stmt in contents.iter_mut().skip(i) {
                collapse_while_loops(stmt);
            }
        }
        Stmt::For(_, _, _, body) => collapse_while_loops(body),
        Stmt::If(_, then_part, else_part, _) => {
            collapse_while_loops(then_part);
            if let Some(stmt) = else_part {
                collapse_while_loops(stmt);
            }
        }
        Stmt::Match(_, cases, default, _) => {
            for case in cases {
                collapse_while_loops(&mut case.body);
            }
            if let Some(stmt) = default {
                collapse_while_loops(stmt);
            }
        }
        Stmt::While(_, body) => collapse_while_loops(body),
        _ => {}
    }
}

fn is_while_loop_sequence(stmts: &[Stmt]) -> bool {
    if let (Stmt::Label(label), Stmt::If(_, then_part, _, _)) = (&stmts[0], &stmts[1]) {
        if let Stmt::Block(contents) = then_part.as_ref() {
            if let Some(Stmt::Goto(other_label)) = contents.last() {
                return label == other_label;
            }
        }
    }
    false
}

pub fn collapse_for_loops(stmt: &mut Stmt) {
    match stmt {
        Stmt::Block(contents) => {
            let mut i = 0;
            while i + 5 < contents.len() {
                if is_for_loop_sequence(&contents[i..i + 6]) {
                    // TODO: Revisit this at some point.
                    //       Blocks should be small so it's not the end of the world, but this is a lot of shifting.
                    let mut targets = contents.drain(i..i + 6).collect_vec();
                    let init = targets.remove(0);
                    let continue_label = targets.remove(1).unwrap_label();
                    let step = targets.remove(1);
                    let (check, mut body, _, break_label) = targets.remove(2).unwrap_if();
                    if let Stmt::Block(contents) = body.as_mut() {
                        contents.pop(); // Remove the jump to the loop check - this is implicit in the source
                    }
                    replace_jumps_with_break_and_continue(&mut body, break_label, continue_label);
                    collapse_for_loops(&mut body);
                    contents.insert(i, Stmt::For(Box::new(init), check, Box::new(step), body));
                }
                collapse_for_loops(&mut contents[i]);
                i += 1;
            }
            for stmt in contents.iter_mut().skip(i) {
                collapse_for_loops(stmt);
            }
        }
        Stmt::For(_, _, _, body) => collapse_for_loops(body),
        Stmt::If(_, then_part, else_part, _) => {
            collapse_for_loops(then_part);
            if let Some(stmt) = else_part {
                collapse_for_loops(stmt);
            }
        }
        Stmt::Match(_, cases, default, _) => {
            for case in cases {
                collapse_for_loops(&mut case.body);
            }
            if let Some(stmt) = default {
                collapse_for_loops(stmt);
            }
        }
        Stmt::While(_, body) => collapse_for_loops(body),
        _ => {}
    }
}

fn is_for_loop_sequence(stmts: &[Stmt]) -> bool {
    if let (
        Stmt::Assign(_, _, _),
        Stmt::Goto(maybe_check_label),
        Stmt::Label(step_label),
        Stmt::Label(check_label),
        Stmt::If(_, then_part, _, _),
    ) = (&stmts[0], &stmts[1], &stmts[2], &stmts[4], &stmts[5])
    {
        // TODO: Should we perform some assertion on the value of the step (stmts[3])?
        if let Stmt::Block(contents) = then_part.as_ref() {
            if let Some(Stmt::Goto(maybe_step_label)) = contents.last() {
                return *maybe_check_label == *check_label && *maybe_step_label == *step_label;
            }
        }
    }
    false
}

pub fn add_match_breaks(stmt: &mut Stmt) {
    match stmt {
        Stmt::Block(contents) => {
            for line in contents {
                add_match_breaks(line);
            }
        }
        Stmt::For(_, _, _, body) => add_match_breaks(body),
        Stmt::If(_, then_part, else_part, _) => {
            add_match_breaks(then_part);
            if let Some(stmt) = else_part {
                add_match_breaks(stmt);
            }
        }
        Stmt::Match(_, cases, default, break_label) => {
            for case in cases {
                add_match_breaks(&mut case.body);
                replace_jumps_with_break_and_continue(&mut case.body, break_label, "");
            }
            if let Some(stmt) = default {
                add_match_breaks(stmt);
                replace_jumps_with_break_and_continue(stmt, break_label, "");
            }
        }
        Stmt::While(_, body) => add_match_breaks(body),
        _ => {}
    }
}

fn replace_jumps_with_break_and_continue<'a>(
    stmt: &mut Stmt<'a>,
    break_label: &'a str,
    continue_label: &'a str,
) {
    match stmt {
        Stmt::Block(contents) => {
            let mut i = 0;
            while i < contents.len() {
                if let Stmt::Goto(label) = &contents[i] {
                    if *label == break_label {
                        contents[i] = Stmt::Break;
                    } else if *label == continue_label {
                        contents[i] = Stmt::Continue;
                    }
                } else {
                    replace_jumps_with_break_and_continue(
                        &mut contents[i],
                        break_label,
                        continue_label,
                    );
                }
                i += 1;
            }
        }
        Stmt::For(_, _, _, body) => {
            replace_jumps_with_break_and_continue(body, break_label, continue_label)
        }
        Stmt::If(_, then_part, else_part, _) => {
            replace_jumps_with_break_and_continue(then_part, break_label, continue_label);
            if let Some(stmt) = else_part {
                replace_jumps_with_break_and_continue(stmt, break_label, continue_label);
            }
        }
        Stmt::Match(_, cases, default, _) => {
            for case in cases {
                replace_jumps_with_break_and_continue(&mut case.body, break_label, continue_label);
            }
            if let Some(stmt) = default {
                replace_jumps_with_break_and_continue(stmt, break_label, continue_label);
            }
        }
        Stmt::While(_, body) => {
            replace_jumps_with_break_and_continue(body, break_label, continue_label)
        }
        _ => {}
    }
}

pub fn collect_var_details(
    stmt: &Stmt,
    arity: usize,
    frame_size: usize,
    global_var_tracker: &mut VarTracker,
) -> Result<VarTracker> {
    let mut var_tracker = VarTracker::new(frame_size);
    for i in 0..arity {
        var_tracker.mark_initialized(i)?;
        var_tracker.mark_used(i)?;
        var_tracker.mark_parameter(i)?;
    }
    collect_var_details_recursive(stmt, &mut var_tracker, global_var_tracker)?;
    Ok(var_tracker)
}

fn collect_var_details_recursive(
    stmt: &Stmt,
    var_tracker: &mut VarTracker,
    global_var_tracker: &mut VarTracker,
) -> Result<()> {
    match stmt {
        Stmt::Assign(op, left, right) => {
            collect_var_details_in_expr_recursive(right, var_tracker, global_var_tracker)?;
            if let Reference::Var(FrameId(frame_id, global)) = left {
                if *global {
                    collect_var_details_in_assign_lhs(*frame_id, *op, global_var_tracker)?;
                } else {
                    collect_var_details_in_assign_lhs(*frame_id, *op, var_tracker)?;
                }
            } else {
                collect_var_details_in_ref(left, var_tracker, global_var_tracker)?;
            }
        }
        Stmt::Block(contents) => {
            for line in contents {
                collect_var_details_recursive(line, var_tracker, global_var_tracker)?;
            }
        }
        Stmt::Expr(e) => collect_var_details_in_expr_recursive(e, var_tracker, global_var_tracker)?,
        Stmt::For(init, check, step, body) => {
            collect_var_details_recursive(init, var_tracker, global_var_tracker)?;
            collect_var_details_in_expr_recursive(check, var_tracker, global_var_tracker)?;
            collect_var_details_recursive(step, var_tracker, global_var_tracker)?;
            collect_var_details_recursive(body, var_tracker, global_var_tracker)?;
        }
        Stmt::If(check, then_part, else_part, _) => {
            collect_var_details_in_expr_recursive(check, var_tracker, global_var_tracker)?;
            collect_var_details_recursive(then_part, var_tracker, global_var_tracker)?;
            if let Some(else_part) = else_part {
                collect_var_details_recursive(else_part, var_tracker, global_var_tracker)?;
            }
        }
        Stmt::Match(switch, cases, default, _) => {
            collect_var_details_in_expr_recursive(switch, var_tracker, global_var_tracker)?;
            for case in cases {
                for check in &case.conditions {
                    collect_var_details_in_expr_recursive(check, var_tracker, global_var_tracker)?;
                }
                collect_var_details_recursive(&case.body, var_tracker, global_var_tracker)?;
            }
            if let Some(default) = default {
                collect_var_details_recursive(default, var_tracker, global_var_tracker)?;
            }
        }
        Stmt::Printf(args) => {
            for arg in args {
                collect_var_details_in_expr_recursive(arg, var_tracker, global_var_tracker)?;
            }
        }
        Stmt::Return(Some(value)) => {
            collect_var_details_in_expr_recursive(value, var_tracker, global_var_tracker)?
        }
        Stmt::While(check, body) => {
            collect_var_details_in_expr_recursive(check, var_tracker, global_var_tracker)?;
            collect_var_details_recursive(body, var_tracker, global_var_tracker)?;
        }
        _ => {}
    }
    Ok(())
}

fn collect_var_details_in_assign_lhs(
    frame_id: usize,
    op: Operator,
    var_tracker: &mut VarTracker,
) -> Result<()> {
    if let Operator::Assign = op {
        if var_tracker.is_initialized(frame_id)? {
            var_tracker.mark_reassigned(frame_id)?;
        } else {
            var_tracker.mark_initialized(frame_id)?;
        }
    }
    Ok(())
}

fn collect_var_details_in_expr_recursive(
    expr: &Expr,
    var_tracker: &mut VarTracker,
    global_var_tracker: &mut VarTracker,
) -> Result<()> {
    match expr {
        Expr::Unary(_, expr) => {
            collect_var_details_in_expr_recursive(expr, var_tracker, global_var_tracker)?
        }
        Expr::Binary(_, left, right) => {
            collect_var_details_in_expr_recursive(left, var_tracker, global_var_tracker)?;
            collect_var_details_in_expr_recursive(right, var_tracker, global_var_tracker)?;
        }
        Expr::Call(_, args) => {
            for arg in args {
                collect_var_details_in_expr_recursive(arg, var_tracker, global_var_tracker)?;
            }
        }
        Expr::Ref(r) => collect_var_details_in_ref(r, var_tracker, global_var_tracker)?,
        Expr::Addr(r) => collect_var_details_in_ref(r, var_tracker, global_var_tracker)?,
        Expr::Inc(_, _, r) => {
            collect_var_details_in_ref(r, var_tracker, global_var_tracker)?;
        }
        Expr::Grouped(e) => {
            collect_var_details_in_expr_recursive(e, var_tracker, global_var_tracker)?
        }
        _ => {}
    }
    Ok(())
}

fn collect_var_details_in_ref(
    reference: &Reference,
    var_tracker: &mut VarTracker,
    global_var_tracker: &mut VarTracker,
) -> Result<()> {
    match reference {
        Reference::Var(FrameId(frame_id, global)) => {
            if *global {
                global_var_tracker.mark_used(*frame_id)?;
            } else {
                var_tracker.mark_used(*frame_id)?;
            }
        }
        Reference::Index(FrameId(frame_id, global), expr) => {
            collect_var_details_in_expr_recursive(expr, var_tracker, global_var_tracker)?;
            if *global {
                global_var_tracker.mark_indexed(*frame_id)?;
                global_var_tracker.mark_used(*frame_id)?;
            } else {
                var_tracker.mark_indexed(*frame_id)?;
                var_tracker.mark_used(*frame_id)?;
            }
        }
        Reference::Dereference(FrameId(frame_id, global), expr) => {
            collect_var_details_in_expr_recursive(expr, var_tracker, global_var_tracker)?;
            if *global {
                global_var_tracker.mark_used(*frame_id)?;
            } else {
                var_tracker.mark_used(*frame_id)?;
            }
        }
    }
    Ok(())
}

pub fn collapse_static_array_inits(stmt: &mut Stmt, vars: &mut VarTracker) -> Result<()> {
    match stmt {
        Stmt::Block(contents) => {
            // Recurse to each statement before trying to collapse anything.
            // Makes the logic a little easier to follow.
            for line in contents.iter_mut() {
                collapse_static_array_inits(line, vars)?;
            }

            let mut i = 0;
            while i < contents.len() {
                let base = i;
                if let Stmt::Assign(
                    Operator::Assign,
                    Reference::Var(FrameId(frame_id, global)),
                    _,
                ) = &contents[i]
                {
                    i += 1;
                    let is_array_start_candidate =
                        !global && vars.is_initialized(*frame_id)? && vars.is_indexed(*frame_id)?;
                    if !is_array_start_candidate {
                        continue;
                    }
                    let mut target_frame_id = FrameId(frame_id + 1, *global);
                    while i < contents.len() {
                        if let Stmt::Assign(Operator::Assign, Reference::Var(frame_id), _) =
                            &contents[i]
                        {
                            if target_frame_id == *frame_id {
                                let is_array_element_candidate = !frame_id.1
                                    && !vars.is_reassigned(frame_id.0)?
                                    && !vars.is_used(frame_id.0)?;
                                if !is_array_element_candidate {
                                    break;
                                }
                                vars.mark_used(target_frame_id.0)?;
                                target_frame_id.0 += 1;
                                i += 1;
                            } else {
                                // If it's not the target frame ID, it's not assigning to the next var in the frame.
                                // This means it can't be part of the array init.
                                break;
                            }
                        } else {
                            // Not an assignment, so we're done.
                            break;
                        }
                    }
                } else {
                    i += 1;
                }

                if i - base > 1 {
                    // Have at least 2 elements, so we can make an array!
                    let elements = contents.drain(base..i).collect_vec();
                    contents.insert(base, create_static_array_init(elements, vars)?);
                    i = base + 1;
                }
            }
        }
        Stmt::For(_, _, _, body) => collapse_static_array_inits(body, vars)?,
        Stmt::If(_, then_part, else_part, _) => {
            collapse_static_array_inits(then_part, vars)?;
            if let Some(stmt) = else_part {
                collapse_static_array_inits(stmt, vars)?;
            }
        }
        Stmt::Match(_, cases, default, _) => {
            for case in cases {
                collapse_static_array_inits(&mut case.body, vars)?;
            }
            if let Some(stmt) = default {
                collapse_static_array_inits(stmt, vars)?;
            }
        }
        Stmt::While(_, body) => collapse_static_array_inits(body, vars)?,
        _ => {}
    }
    Ok(())
}

fn create_static_array_init<'a>(
    mut assigns: Vec<Stmt<'a>>,
    vars: &mut VarTracker,
) -> Result<Stmt<'a>> {
    let (_, left, right) = assigns.remove(0).unwrap_assign();
    let frame_id = left.frame_id();
    vars.set_array_length(frame_id.0, assigns.len() + 1)?;
    vars.mark_static_array(frame_id.0)?;
    let mut elements = vec![right];
    for element in assigns.into_iter() {
        let (_, _, right) = element.unwrap_assign();
        elements.push(right);
    }
    Ok(Stmt::Assign(
        Operator::Assign,
        left,
        Expr::StaticArrayInit(elements),
    ))
}

pub fn inject_global_var_declarations(script: &mut Script, requests: &[DeclarationRequest]) {
    let inits = requests
        .iter()
        .map(|request| match request {
            DeclarationRequest::Array(index, count) => {
                Decl::GlobalVarDecl(*index, Some(*count))
            }
            DeclarationRequest::Var(index) => Decl::GlobalVarDecl(*index, None),
        })
        .collect_vec();
    script.0.splice(0..0, inits);
}

pub fn inject_var_declarations(block: &mut Stmt, requests: &[DeclarationRequest]) {
    if let Stmt::Block(contents) = block {
        let inits = requests
            .iter()
            .map(|request| match request {
                DeclarationRequest::Array(index, count) => {
                    Stmt::VarDecl(*index, Some(*count))
                }
                DeclarationRequest::Var(index) => Stmt::VarDecl(*index, None),
            })
            .collect_vec();
        contents.splice(0..0, inits);
    } else {
        panic!("bug - trying to add statements to a block, but input is not a block");
    }
}
