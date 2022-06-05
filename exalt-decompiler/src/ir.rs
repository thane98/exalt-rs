use derive_more::Unwrap;
use exalt_ast::{Notation, Operator};

use anyhow::{bail, Result};
use itertools::Itertools;
use std::borrow::Cow;
use std::fmt::Write;

use crate::IrTransform;

#[derive(Clone, PartialEq)]
pub enum Literal<'a> {
    Int(i32),
    Float(f32),
    Str(Cow<'a, str>),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct FrameId(pub usize, pub bool);

#[derive(Clone, PartialEq)]
pub enum Reference<'a> {
    Var(FrameId),
    Index(FrameId, Box<Expr<'a>>),
    Dereference(FrameId, Box<Expr<'a>>),
}

impl<'a> Reference<'a> {
    pub fn frame_id(&self) -> FrameId {
        match self {
            Reference::Var(frame_id) => *frame_id,
            Reference::Index(frame_id, _) => *frame_id,
            Reference::Dereference(frame_id, _) => *frame_id,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Expr<'a> {
    Literal(Literal<'a>),
    Unary(Operator, Box<Expr<'a>>),
    Binary(Operator, Box<Expr<'a>>, Box<Expr<'a>>),
    Call(Cow<'a, str>, Vec<Expr<'a>>),
    Ref(Reference<'a>),
    Addr(Reference<'a>),
    Inc(Operator, Notation, Reference<'a>),
    Grouped(Box<Expr<'a>>),
    StaticArrayInit(Vec<Expr<'a>>),
}

pub struct Case<'a> {
    pub conditions: Vec<Expr<'a>>,
    pub body: Stmt<'a>,
}

#[derive(Unwrap)]
pub enum Stmt<'a> {
    Assign(Operator, Reference<'a>, Expr<'a>),
    Block(Vec<Stmt<'a>>),
    Break,
    Continue,
    Expr(Expr<'a>),
    For(Box<Stmt<'a>>, Expr<'a>, Box<Stmt<'a>>, Box<Stmt<'a>>),
    Goto(&'a str),
    If(Expr<'a>, Box<Stmt<'a>>, Option<Box<Stmt<'a>>>, &'a str),
    Label(&'a str),
    Match(Expr<'a>, Vec<Case<'a>>, Option<Box<Stmt<'a>>>, &'a str),
    Printf(Vec<Expr<'a>>),
    Return(Option<Expr<'a>>),
    VarDecl(usize, Option<usize>),
    While(Expr<'a>, Box<Stmt<'a>>),
    Yield,
}

pub enum Annotation<'a> {
    NoDefaultReturn,
    Prefix(&'a [u8]),
    Suffix(&'a [u8]),
    Unknown(u8),
}

pub enum Decl<'a> {
    Callback(Vec<Annotation<'a>>, u8, Vec<Literal<'a>>, Stmt<'a>),
    Function(Vec<Annotation<'a>>, String, usize, Stmt<'a>),
    GlobalVarDecl(usize, Option<usize>),
}

impl<'a> Decl<'a> {
    pub fn append_annotation(&mut self, annotation: Annotation<'a>) {
        match self {
            Decl::Callback(annotations, _, _, _) => annotations.push(annotation),
            Decl::Function(annotations, _, _, _) => annotations.push(annotation),
            _ => panic!("bug - this decl does not accept annotations"),
        }
    }
}

pub struct Script<'a>(pub Vec<Decl<'a>>);

pub fn pretty_print(script: &Script, transform: &IrTransform, includes: &[String]) -> Result<String> {
    let mut sb = String::new();
    for inc in includes {
        writeln!(sb, "include {};", inc)?;
    }
    if !includes.is_empty() {
        sb.push('\n');
    }
    let (vars, functions): (Vec<&Decl>, Vec<&Decl>) = script.0.iter().partition(|d| matches!(d, Decl::GlobalVarDecl(_, _)));
    for decl in &vars {
        pretty_print_decl(&mut sb, decl, transform)?;
        sb.push('\n');
    }
    if !vars.is_empty() {
        sb.push('\n');
    }
    for decl in functions {
        pretty_print_decl(&mut sb, decl, transform)?;
        sb.push_str("\n\n");
    }
    Ok(sb)
}

fn pretty_print_decl(sb: &mut String, decl: &Decl, transform: &IrTransform) -> Result<()> {
    match decl {
        Decl::Callback(annotations, event, args, body) => {
            for annotation in annotations {
                pretty_print_annotation(sb, annotation)?;
                sb.push('\n');
            }
            sb.push_str("callback[");
            if let Some(name) = transform.transform_event((*event).into()) {
                write!(sb, "{}", name)?;
            } else {
                write!(sb, "0x{:X}", event)?;
            }
            sb.push_str("](");
            for (i, arg) in args.iter().enumerate() {
                pretty_print_literal(sb, arg, transform)?;
                if i + 1 < args.len() {
                    sb.push_str(", ");
                }
            }
            sb.push_str(") ");
            pretty_print_stmt(sb, body, 0, transform)?;
        }
        Decl::Function(annotations, name, arity, body) => {
            for annotation in annotations {
                pretty_print_annotation(sb, annotation)?;
                sb.push('\n');
            }
            write!(sb, "def {}(", transform.transform_function_name(name).unwrap_or(name))?;
            for i in 0..*arity {
                pretty_print_var(sb, FrameId(i, false))?;
                if i + 1 < *arity {
                    sb.push_str(", ");
                }
            }
            sb.push_str(") ");
            pretty_print_stmt(sb, body, 0, transform)?;
        }
        Decl::GlobalVarDecl(base, count) => {
            sb.push_str("let ");
            pretty_print_var(sb, FrameId(*base, true))?;
            if let Some(count) = count {
                write!(sb, "[{}]", count)?;
            }
            sb.push(';');
        }
    }
    Ok(())
}

fn pretty_print_annotation(sb: &mut String, annotation: &Annotation) -> Result<()> {
    sb.push('@');
    match annotation {
        Annotation::NoDefaultReturn => sb.push_str("NoDefaultReturn"),
        Annotation::Prefix(v) => write!(
            sb,
            "Prefix({})",
            v.iter().map(|v| format!("0x{:X}", v)).join(", ")
        )?,
        Annotation::Suffix(v) => write!(
            sb,
            "Suffix({})",
            v.iter().map(|v| format!("0x{:X}", v)).join(", ")
        )?,
        Annotation::Unknown(v) => write!(sb, "Unknown(0x{:X})", v)?,
    }
    Ok(())
}

fn pretty_print_stmt(sb: &mut String, stmt: &Stmt, indent: usize, transform: &IrTransform) -> Result<()> {
    match stmt {
        Stmt::Assign(op, left, right) => {
            pretty_print_ref(sb, left, indent, transform)?;
            write!(sb, " {} ", op)?;
            pretty_print_expr(sb, right, indent, transform)?;
            sb.push(';');
        }
        Stmt::Block(lines) => {
            if lines.is_empty() {
                sb.push_str("{}");
            } else {
                sb.push_str("{\n");
                for line in lines {
                    add_indent(sb, indent + 1);
                    pretty_print_stmt(sb, line, indent + 1, transform)?;
                    sb.push('\n');
                }
                add_indent(sb, indent);
                sb.push('}');
            }
        }
        Stmt::Break => sb.push_str("break;"),
        Stmt::Continue => sb.push_str("continue;"),
        Stmt::Expr(expr) => {
            pretty_print_expr(sb, expr, indent, transform)?;
            sb.push(';');
        }
        Stmt::For(init, check, step, body) => {
            sb.push_str("for (");
            pretty_print_stmt(sb, init, indent, transform)?;
            sb.push(' ');
            pretty_print_expr(sb, check, indent, transform)?;
            sb.push_str("; ");
            match step.as_ref() {
                Stmt::Assign(op, left, right) => {
                    pretty_print_ref(sb, left, indent, transform)?;
                    write!(sb, " {} ", op)?;
                    pretty_print_expr(sb, right, indent, transform)?;
                }
                Stmt::Expr(e) => pretty_print_expr(sb, e, indent, transform)?,
                _ => bail!("unexpected step part in for loop"),
            }
            sb.push_str(") ");
            pretty_print_stmt(sb, body, indent, transform)?;
        }
        Stmt::Goto(label) => write!(sb, "goto {};", label)?,
        Stmt::If(check, then_part, else_part, _) => {
            sb.push_str("if (");
            pretty_print_expr(sb, check, indent, transform)?;
            sb.push_str(") ");
            pretty_print_stmt(sb, then_part, indent, transform)?;
            if let Some(stmt) = else_part {
                sb.push_str(" else ");
                pretty_print_stmt(sb, stmt, indent, transform)?;
            }
        }
        Stmt::Label(label) => write!(sb, "label {};", label)?,
        Stmt::Match(switch, cases, default, _) => {
            sb.push_str("match (");
            pretty_print_expr(sb, switch, indent, transform)?;
            sb.push_str(") {\n");
            for case in cases {
                add_indent(sb, indent + 1);
                for (i, check) in case.conditions.iter().enumerate() {
                    pretty_print_expr(sb, check, indent, transform)?;
                    if i + 1 < case.conditions.len() {
                        sb.push_str(", ");
                    }
                }
                sb.push_str(" -> ");
                pretty_print_stmt(sb, &case.body, indent + 1, transform)?;
                sb.push('\n');
            }
            if let Some(default) = default {
                add_indent(sb, indent + 1);
                sb.push_str("else -> ");
                pretty_print_stmt(sb, default, indent + 1, transform)?;
                sb.push('\n');
            }
            add_indent(sb, indent);
            sb.push('}');
        }
        Stmt::Printf(args) => {
            sb.push_str("printf(");
            for i in 0..args.len() {
                pretty_print_expr(sb, &args[i], indent, transform)?;
                if i + 1 < args.len() {
                    sb.push_str(", ");
                }
            }
            sb.push_str(");");
        }
        Stmt::Return(value) => {
            if let Some(value) = value {
                sb.push_str("return ");
                pretty_print_expr(sb, value, indent, transform)?;
                sb.push(';');
            } else {
                sb.push_str("return;");
            }
        }
        Stmt::VarDecl(frame_id, count) => {
            sb.push_str("let ");
            pretty_print_var(sb, FrameId(*frame_id, false))?;
            if let Some(count) = count {
                write!(sb, "[{}]", count)?;
            }
            sb.push(';');
        }
        Stmt::While(check, body) => {
            sb.push_str("while (");
            pretty_print_expr(sb, check, indent, transform)?;
            sb.push_str(") ");
            pretty_print_stmt(sb, body, indent, transform)?;
        }
        Stmt::Yield => sb.push_str("yield;"),
    }
    Ok(())
}

fn add_indent(sb: &mut String, indent: usize) {
    for _ in 0..indent {
        sb.push_str("    ");
    }
}

fn pretty_print_expr(sb: &mut String, expr: &Expr, indent: usize, transform: &IrTransform) -> Result<()> {
    match expr {
        Expr::Literal(l) => pretty_print_literal(sb, l, transform)?,
        Expr::Unary(op, operand) => {
            write!(sb, "{}", op)?;
            pretty_print_expr(sb, operand, indent, transform)?;
        }
        Expr::Binary(op, left, right) => {
            pretty_print_expr(sb, left, indent, transform)?;
            write!(sb, " {} ", op)?;
            pretty_print_expr(sb, right, indent, transform)?;
        }
        Expr::Call(name, args) => {
            sb.push_str(transform.transform_function_name(name).unwrap_or(name));
            sb.push('(');
            for i in 0..args.len() {
                pretty_print_expr(sb, &args[i], indent, transform)?;
                if i + 1 < args.len() {
                    sb.push_str(", ");
                }
            }
            sb.push(')');
        }
        Expr::Ref(r) => pretty_print_ref(sb, r, indent, transform)?,
        Expr::Addr(r) => {
            sb.push('&');
            pretty_print_ref(sb, r, indent, transform)?;
        }
        Expr::Inc(op, notation, operand) => {
            if let Notation::Prefix = notation {
                write!(sb, "{}", op)?;
            }
            pretty_print_ref(sb, operand, indent, transform)?;
            if let Notation::Postfix = notation {
                write!(sb, "{}", op)?;
            }
        }
        Expr::Grouped(e) => {
            sb.push('(');
            pretty_print_expr(sb, e, indent, transform)?;
            sb.push(')');
        }
        Expr::StaticArrayInit(entries) => {
            sb.push('[');
            if entries.len() < 5 {
                for (i, entry) in entries.iter().enumerate() {
                    pretty_print_expr(sb, entry, indent, transform)?;
                    if i + 1 < entries.len() {
                        sb.push_str(", ");
                    }
                }
            } else {
                for i in (0..entries.len()).step_by(4) {
                    sb.push('\n');
                    add_indent(sb, indent + 1);
                    for j in 0..(4.min(entries.len() - i)) {
                        pretty_print_expr(sb, &entries[i + j], indent + 1, transform)?;
                        sb.push_str(", ");
                    }
                }
                sb.push('\n');
                add_indent(sb, indent);
            }
            sb.push(']');
        }
    }
    Ok(())
}

fn pretty_print_literal(sb: &mut String, literal: &Literal, transform: &IrTransform) -> Result<()> {
    match literal {
        Literal::Int(v) => write!(sb, "{}", v)?,
        Literal::Float(v) => if v.fract() == 0.0 {
            write!(sb, "{:.1}", v)?
        } else {
            write!(sb, "{:.}", v)?
        },
        Literal::Str(v) => if let Some(transformed_value) = transform.transform_string(v) {
            sb.push_str(transformed_value) // TODO: Unescape?
        } else {
            write!(sb, "\"{}\"", v)? // TODO: Unescape?
        }
    }
    Ok(())
}

fn pretty_print_ref(sb: &mut String, reference: &Reference, indent: usize, transform: &IrTransform) -> Result<()> {
    match reference {
        Reference::Var(frame_id) => pretty_print_var(sb, *frame_id)?,
        Reference::Index(frame_id, index) => {
            pretty_print_var(sb, *frame_id)?;
            sb.push('[');
            pretty_print_expr(sb, index, indent, transform)?;
            sb.push(']');
        }
        Reference::Dereference(frame_id, index) => if is_useless_index(index) {
            sb.push('*');
            pretty_print_var(sb, *frame_id)?;
        } else {
            sb.push('*');
            pretty_print_var(sb, *frame_id)?;
            sb.push('[');
            pretty_print_expr(sb, index, indent, transform)?;
            sb.push(']');
        }
    }
    Ok(())
}

fn is_useless_index(index: &Expr) -> bool {
    if let Expr::Literal(Literal::Int(v)) = index {
        return *v == 0;
    }
    false
}

// TODO: Set up a better system for picking var names?
fn pretty_print_var(sb: &mut String, frame_id: FrameId) -> Result<()> {
    if frame_id.1 {
        write!(sb, "g_v{}", frame_id.0)?;
    } else {
        write!(sb, "v{}", frame_id.0)?;
    }
    Ok(())
}
