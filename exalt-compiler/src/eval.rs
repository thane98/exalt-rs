use crate::reporting::SemanticError;
use crate::symbol::{SymbolTable, Variable};
use exalt_ast::surface::{Expr, Identifier, Ref};
use exalt_ast::{Literal, Location, Operator};

type Result<T> = std::result::Result<T, SemanticError>;

/// Evaluate a constant expression like 2 + 4 * 3
/// Supports constants and enums which are already defined
pub(crate) fn evaluate_const_expr(symbol_table: &SymbolTable, expr: &Expr) -> Result<Literal> {
    match expr {
        Expr::Array(l, _) => Err(SemanticError::ExpectedConstExpr(l.clone())),
        Expr::Literal(_, l) => Ok(l.clone()),
        Expr::EnumAccess(_, name, variant) => evaluate_enum_access(symbol_table, name, variant),
        Expr::Unary(l, e, o) => evaluate_const_unary(symbol_table, l, e, *o),
        Expr::Binary(loc, l, o, r) => evaluate_const_binary(symbol_table, loc, l, *o, r),
        Expr::FunctionCall(l, _, _) => Err(SemanticError::ExpectedConstExpr(l.clone())),
        Expr::Ref(l, r) => evaluate_const_ref(symbol_table, l, r),
        Expr::Grouped(_, e) => evaluate_const_expr(symbol_table, e),
        Expr::Increment(l, _, _, _) => Err(SemanticError::ExpectedConstExpr(l.clone())),
        Expr::AddressOf(l, _) => Err(SemanticError::ExpectedConstExpr(l.clone())),
    }
}

pub(crate) fn evaluate_enum_access(
    symbol_table: &SymbolTable,
    name: &Identifier,
    variant: &Identifier,
) -> Result<Literal> {
    match symbol_table.lookup_enum(&name.value) {
        Some(e) => match e.borrow().variants.get(&variant.value) {
            Some(v) => Ok(v.value.clone()),
            None => Err(SemanticError::UndefinedVariant(variant.clone())),
        },
        None => Err(SemanticError::UndefinedEnum(name.clone())),
    }
}

fn evaluate_const_unary(
    symbol_table: &SymbolTable,
    location: &Location,
    expr: &Expr,
    op: Operator,
) -> Result<Literal> {
    let operand = evaluate_const_expr(symbol_table, expr)?;
    match (operand, op) {
        (Literal::Int(i), Operator::LogicalNot) => Ok(Literal::Int(if i == 0 { 0 } else { 1 })),
        (Literal::Int(i), Operator::BitwiseNot) => Ok(Literal::Int(!i)),
        (Literal::Int(i), Operator::Negate) => Ok(Literal::Int(-i)),
        (Literal::Float(f), Operator::FloatNegate) => Ok(Literal::Float(-f)),
        (operand, _) => Err(SemanticError::IncompatibleOperator(
            location.clone(),
            operand.data_type().name(),
            op,
        )),
    }
}

fn evaluate_const_binary(
    symbol_table: &SymbolTable,
    location: &Location,
    left: &Expr,
    op: Operator,
    right: &Expr,
) -> Result<Literal> {
    let left = evaluate_const_expr(symbol_table, left)?;
    let right = evaluate_const_expr(symbol_table, right)?;
    if left.data_type() != right.data_type() {
        return Err(SemanticError::IncompatibleOperands(
            location.clone(),
            left.data_type().name(),
            right.data_type().name(),
        ));
    }
    match (left, op, right) {
        (Literal::Int(l), Operator::Add, Literal::Int(r)) => Ok(Literal::Int(l + r)),
        (Literal::Int(l), Operator::Subtract, Literal::Int(r)) => Ok(Literal::Int(l - r)),
        (Literal::Int(l), Operator::Multiply, Literal::Int(r)) => Ok(Literal::Int(l * r)),
        (Literal::Int(l), Operator::Divide, Literal::Int(r)) => {
            if r == 0 {
                Err(SemanticError::DivideByZero(location.clone()))
            } else {
                Ok(Literal::Int(l / r))
            }
        }
        (Literal::Int(l), Operator::Modulo, Literal::Int(r)) => {
            if r == 0 {
                Err(SemanticError::DivideByZero(location.clone()))
            } else {
                Ok(Literal::Int(l % r))
            }
        }
        (Literal::Int(l), Operator::LeftShift, Literal::Int(r)) => Ok(Literal::Int(l << r)),
        (Literal::Int(l), Operator::RightShift, Literal::Int(r)) => Ok(Literal::Int(l >> r)),
        (Literal::Int(l), Operator::BitwiseAnd, Literal::Int(r)) => Ok(Literal::Int(l & r)),
        (Literal::Int(l), Operator::BitwiseOr, Literal::Int(r)) => Ok(Literal::Int(l | r)),
        (Literal::Int(l), Operator::Xor, Literal::Int(r)) => Ok(Literal::Int(l ^ r)),
        (Literal::Int(l), Operator::LogicalOr, Literal::Int(r)) => Ok(if l != 0 || r != 0 {
            Literal::Int(1)
        } else {
            Literal::Int(0)
        }),
        (Literal::Int(l), Operator::LogicalAnd, Literal::Int(r)) => Ok(if l != 0 && r != 0 {
            Literal::Int(1)
        } else {
            Literal::Int(0)
        }),
        (Literal::Float(l), Operator::FloatAdd, Literal::Float(r)) => Ok(Literal::Float(l + r)),
        (Literal::Float(l), Operator::FloatSubtract, Literal::Float(r)) => {
            Ok(Literal::Float(l - r))
        }
        (Literal::Float(l), Operator::FloatMultiply, Literal::Float(r)) => {
            Ok(Literal::Float(l * r))
        }
        (Literal::Float(l), Operator::FloatDivide, Literal::Float(r)) => {
            if r == 0.0 {
                Err(SemanticError::DivideByZero(location.clone()))
            } else {
                Ok(Literal::Float(l / r))
            }
        }
        (left, op, _) => Err(SemanticError::IncompatibleOperator(
            location.clone(),
            left.data_type().name(),
            op,
        )),
    }
}

fn evaluate_const_ref(
    symbol_table: &SymbolTable,
    location: &Location,
    expr: &Ref,
) -> Result<Literal> {
    match expr {
        Ref::Var(i) => match symbol_table.lookup_variable(&i.value) {
            Some(v) => match v {
                Variable::Const(c) => Ok(c.borrow().value.clone()),
                Variable::Var(_) => Err(SemanticError::ExpectedConstExpr(location.clone())),
            },
            None => Err(SemanticError::UndefinedVariable(i.clone())),
        },
        _ => Err(SemanticError::ExpectedConstExpr(location.clone())),
    }
}
