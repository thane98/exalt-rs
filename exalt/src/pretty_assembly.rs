use std::collections::HashMap;

use crate::common::PrettyFunctionData;
use crate::{FunctionData, Opcode};
use anyhow::Context;
use lazy_static::lazy_static;
use maplit::hashmap;

macro_rules! unprettify_unary_int {
    ($line:ident, $variant:ident, $t:ident, $instr:tt) => {
        if $line.len() == 2 {
            let value = $t::from_str_radix($line[1], 10)?;
            Ok(Opcode::$variant(value))
        } else {
            Err(anyhow::anyhow!("$instr has incorrect number of operands"))
        }
    };
}

macro_rules! unprettify_string {
    ($line:ident, $variant:ident) => {
        if $line.len() == 2 {
            Ok(Opcode::$variant($line[1].to_string()))
        } else {
            Err(anyhow::anyhow!("Incorrect number of operands"))
        }
    };
}

lazy_static! {
    pub static ref UNPRETTIFY_DISPATCH_TABLE: HashMap<&'static str, fn(&[&str]) -> anyhow::Result<Opcode>> = {
        hashmap! {
            "nop" => unprettify_nop as fn(&[&str]) -> anyhow::Result<Opcode>,
            "val" => unprettify_var_load as fn(&[&str]) -> anyhow::Result<Opcode>,
            "valx" => unprettify_arr_load as fn(&[&str]) -> anyhow::Result<Opcode>,
            "valy" => unprettify_ptr_load as fn(&[&str]) -> anyhow::Result<Opcode>,
            "ref" => unprettify_var_addr as fn(&[&str]) -> anyhow::Result<Opcode>,
            "refx" => unprettify_arr_addr as fn(&[&str]) -> anyhow::Result<Opcode>,
            "refy" => unprettify_ptr_addr as fn(&[&str]) -> anyhow::Result<Opcode>,
            "gval" => unprettify_global_var_load as fn(&[&str]) -> anyhow::Result<Opcode>,
            "gvalx" => unprettify_global_arr_load as fn(&[&str]) -> anyhow::Result<Opcode>,
            "gvaly" => unprettify_global_ptr_load as fn(&[&str]) -> anyhow::Result<Opcode>,
            "gref" => unprettify_global_var_addr as fn(&[&str]) -> anyhow::Result<Opcode>,
            "grefx" => unprettify_global_arr_addr as fn(&[&str]) -> anyhow::Result<Opcode>,
            "grefy" => unprettify_global_ptr_addr as fn(&[&str]) -> anyhow::Result<Opcode>,
            "int" => unprettify_int_load as fn(&[&str]) -> anyhow::Result<Opcode>,
            "string" => unprettify_str_load as fn(&[&str]) -> anyhow::Result<Opcode>,
            "float" => unprettify_float_load as fn(&[&str]) -> anyhow::Result<Opcode>,
            "deref" => unprettify_dereference as fn(&[&str]) -> anyhow::Result<Opcode>,
            "consume" => unprettify_consume as fn(&[&str]) -> anyhow::Result<Opcode>,
            "store" => unprettify_complete_assign as fn(&[&str]) -> anyhow::Result<Opcode>,
            "cint" => unprettify_fix as fn(&[&str]) -> anyhow::Result<Opcode>,
            "cfloat" => unprettify_float as fn(&[&str]) -> anyhow::Result<Opcode>,
            "add" => unprettify_add as fn(&[&str]) -> anyhow::Result<Opcode>,
            "fadd" => unprettify_float_add as fn(&[&str]) -> anyhow::Result<Opcode>,
            "sub" => unprettify_subtract as fn(&[&str]) -> anyhow::Result<Opcode>,
            "fsub" => unprettify_float_subtract as fn(&[&str]) -> anyhow::Result<Opcode>,
            "mul" => unprettify_multiply as fn(&[&str]) -> anyhow::Result<Opcode>,
            "fmul" => unprettify_float_multiply as fn(&[&str]) -> anyhow::Result<Opcode>,
            "div" => unprettify_divide as fn(&[&str]) -> anyhow::Result<Opcode>,
            "fdiv" => unprettify_float_divide as fn(&[&str]) -> anyhow::Result<Opcode>,
            "mod" => unprettify_modulo as fn(&[&str]) -> anyhow::Result<Opcode>,
            "neg" => unprettify_negate as fn(&[&str]) -> anyhow::Result<Opcode>,
            "fneg" => unprettify_float_negate as fn(&[&str]) -> anyhow::Result<Opcode>,
            "mvn" => unprettify_binary_not as fn(&[&str]) -> anyhow::Result<Opcode>,
            "not" => unprettify_logical_not as fn(&[&str]) -> anyhow::Result<Opcode>,
            "orr" => unprettify_binary_or as fn(&[&str]) -> anyhow::Result<Opcode>,
            "and" => unprettify_binary_and as fn(&[&str]) -> anyhow::Result<Opcode>,
            "xor" => unprettify_xor as fn(&[&str]) -> anyhow::Result<Opcode>,
            "lsl" => unprettify_left_shift as fn(&[&str]) -> anyhow::Result<Opcode>,
            "lsr" => unprettify_right_shift as fn(&[&str]) -> anyhow::Result<Opcode>,
            "eq" => unprettify_equal as fn(&[&str]) -> anyhow::Result<Opcode>,
            "feq" => unprettify_float_equal as fn(&[&str]) -> anyhow::Result<Opcode>,
            "exlcall" => unprettify_exlcall as fn(&[&str]) -> anyhow::Result<Opcode>,
            "ne" => unprettify_not_equal as fn(&[&str]) -> anyhow::Result<Opcode>,
            "fne" => unprettify_float_not_equal as fn(&[&str]) -> anyhow::Result<Opcode>,
            "nop0x3d" => unprettify_nop0x3d as fn(&[&str]) -> anyhow::Result<Opcode>,
            "lt" => unprettify_less_than as fn(&[&str]) -> anyhow::Result<Opcode>,
            "flt" => unprettify_float_less_than as fn(&[&str]) -> anyhow::Result<Opcode>,
            "le" => unprettify_less_than_equal_to as fn(&[&str]) -> anyhow::Result<Opcode>,
            "fle" => unprettify_float_less_than_equal_to as fn(&[&str]) -> anyhow::Result<Opcode>,
            "gt" => unprettify_greater_than as fn(&[&str]) -> anyhow::Result<Opcode>,
            "fgt" => unprettify_float_greater_than as fn(&[&str]) -> anyhow::Result<Opcode>,
            "ge" => unprettify_greater_than_equal_to as fn(&[&str]) -> anyhow::Result<Opcode>,
            "fge" => unprettify_float_greater_than_equal_to as fn(&[&str]) -> anyhow::Result<Opcode>,
            "call.loc" => unprettify_call_by_id as fn(&[&str]) -> anyhow::Result<Opcode>,
            "call" => unprettify_call_by_name as fn(&[&str]) -> anyhow::Result<Opcode>,
            "setret" => unprettify_set_return as fn(&[&str]) -> anyhow::Result<Opcode>,
            "b" => unprettify_jump as fn(&[&str]) -> anyhow::Result<Opcode>,
            "by" => unprettify_jump_not_zero as fn(&[&str]) -> anyhow::Result<Opcode>,
            "bky" => unprettify_or as fn(&[&str]) -> anyhow::Result<Opcode>,
            "bn" => unprettify_jump_zero as fn(&[&str]) -> anyhow::Result<Opcode>,
            "bkn" => unprettify_and as fn(&[&str]) -> anyhow::Result<Opcode>,
            "yield" => unprettify_yield as fn(&[&str]) -> anyhow::Result<Opcode>,
            "printf" => unprettify_format as fn(&[&str]) -> anyhow::Result<Opcode>,
            "inc" => unprettify_inc as fn(&[&str]) -> anyhow::Result<Opcode>,
            "dec" => unprettify_dec as fn(&[&str]) -> anyhow::Result<Opcode>,
            "dup" => unprettify_copy as fn(&[&str]) -> anyhow::Result<Opcode>,
            "retn" => unprettify_return_false as fn(&[&str]) -> anyhow::Result<Opcode>,
            "rety" => unprettify_return_true as fn(&[&str]) -> anyhow::Result<Opcode>,
            "label" => unprettify_label as fn(&[&str]) -> anyhow::Result<Opcode>,
            "eqstr" => unprettify_string_equals as fn(&[&str]) -> anyhow::Result<Opcode>,
            "nestr" => unprettify_string_not_equals as fn(&[&str]) -> anyhow::Result<Opcode>,
            "ret" => unprettify_return as fn(&[&str]) -> anyhow::Result<Opcode>,
            "nop0x40" => unprettify_nop0x40 as fn(&[&str]) -> anyhow::Result<Opcode>,
            "assign" => unprettify_assign as fn(&[&str]) -> anyhow::Result<Opcode>,
        }
    };
}

fn prettify_opcode(op: &Opcode) -> String {
    match op {
        Opcode::Done => "nop".to_string(),
        Opcode::VarLoad(v) => format!("val {}", v),
        Opcode::ArrLoad(v) => format!("valx {}", v),
        Opcode::PtrLoad(v) => format!("valy {}", v),
        Opcode::VarAddr(v) => format!("ref {}", v),
        Opcode::ArrAddr(v) => format!("refx {}", v),
        Opcode::PtrAddr(v) => format!("refy {}", v),
        Opcode::GlobalVarLoad(v) => format!("gval {}", v),
        Opcode::GlobalArrLoad(v) => format!("gvalx {}", v),
        Opcode::GlobalPtrLoad(v) => format!("gvaly {}", v),
        Opcode::GlobalVarAddr(v) => format!("gref {}", v),
        Opcode::GlobalArrAddr(v) => format!("grefx {}", v),
        Opcode::GlobalPtrAddr(v) => format!("grefy {}", v),
        Opcode::IntLoad(v) => format!("int {}", v),
        Opcode::StrLoad(v) => format!("string {}", v),
        Opcode::FloatLoad(v) => format!("float {}", v),
        Opcode::Dereference => "deref".to_string(),
        Opcode::Consume => "consume".to_string(),
        Opcode::CompleteAssign => "store".to_string(),
        Opcode::Fix => "cint".to_string(),
        Opcode::Float => "cfloat".to_string(),
        Opcode::Add => "add".to_string(),
        Opcode::FloatAdd => "fadd".to_string(),
        Opcode::Subtract => "sub".to_string(),
        Opcode::FloatSubtract => "fsub".to_string(),
        Opcode::Multiply => "mul".to_string(),
        Opcode::FloatMultiply => "fmul".to_string(),
        Opcode::Divide => "div".to_string(),
        Opcode::FloatDivide => "fdiv".to_string(),
        Opcode::Modulo => "mod".to_string(),
        Opcode::IntNegate => "neg".to_string(),
        Opcode::FloatNegate => "fneg".to_string(),
        Opcode::BinaryNot => "mvn".to_string(),
        Opcode::LogicalNot => "not".to_string(),
        Opcode::BinaryOr => "orr".to_string(),
        Opcode::BinaryAnd => "and".to_string(),
        Opcode::Xor => "xor".to_string(),
        Opcode::LeftShift => "lsl".to_string(),
        Opcode::RightShift => "lsr".to_string(),
        Opcode::Equal => "eq".to_string(),
        Opcode::FloatEqual => "feq".to_string(),
        Opcode::Exlcall => "exlcall".to_string(),
        Opcode::NotEqual => "ne".to_string(),
        Opcode::FloatNotEqual => "fne".to_string(),
        Opcode::Nop0x3D => "nop0x3d".to_string(),
        Opcode::LessThan => "lt".to_string(),
        Opcode::FloatLessThan => "flt".to_string(),
        Opcode::LessThanEqualTo => "le".to_string(),
        Opcode::FloatLessThanEqualTo => "fle".to_string(),
        Opcode::GreaterThan => "gt".to_string(),
        Opcode::FloatGreaterThan => "fgt".to_string(),
        Opcode::GreaterThanEqualTo => "ge".to_string(),
        Opcode::FloatGreaterThanEqualTo => "fge".to_string(),
        Opcode::CallById(v) => format!("call.loc {}", v),
        Opcode::CallByName(n, a) => format!("call {} {}", n, a),
        Opcode::SetReturn => format!("setret"),
        Opcode::Jump(v) => format!("b {}", v),
        Opcode::JumpNotZero(v) => format!("by {}", v),
        Opcode::Or(v) => format!("bky {}", v),
        Opcode::JumpZero(v) => format!("bn {}", v),
        Opcode::And(v) => format!("bkn {}", v),
        Opcode::Yield => "yield".to_string(),
        Opcode::Format(v) => format!("printf {}", v),
        Opcode::Inc => "inc".to_string(),
        Opcode::Dec => "dec".to_string(),
        Opcode::Copy => "dup".to_string(),
        Opcode::ReturnFalse => "retn".to_string(),
        Opcode::ReturnTrue => "rety".to_string(),
        Opcode::Label(v) => format!("label {}", v),
        Opcode::StringEquals => "eqstr".to_string(),
        Opcode::StringNotEquals => "nestr".to_string(),
        Opcode::Return => "ret".to_string(),
        Opcode::Nop0x40 => "nop0x40".to_string(),
        Opcode::Assign => "assign".to_string(),
    }
}

fn unprettify_nop(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Done)
}

fn unprettify_var_load(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, VarLoad, u16, "val")
}

fn unprettify_arr_load(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, ArrLoad, u16, "valx")
}

fn unprettify_ptr_load(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, PtrLoad, u16, "valy")
}

fn unprettify_var_addr(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, VarAddr, u16, "ref")
}

fn unprettify_arr_addr(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, ArrAddr, u16, "refx")
}

fn unprettify_ptr_addr(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, PtrAddr, u16, "refy")
}

fn unprettify_global_var_load(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, GlobalVarLoad, u16, "val")
}

fn unprettify_global_arr_load(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, GlobalArrLoad, u16, "valx")
}

fn unprettify_global_ptr_load(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, GlobalPtrLoad, u16, "valy")
}

fn unprettify_global_var_addr(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, GlobalVarAddr, u16, "ref")
}

fn unprettify_global_arr_addr(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, GlobalArrAddr, u16, "refx")
}

fn unprettify_global_ptr_addr(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, GlobalPtrAddr, u16, "refy")
}

fn unprettify_int_load(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, IntLoad, i32, "int")
}

fn unprettify_str_load(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_string!(line, StrLoad)
}

fn unprettify_float_load(line: &[&str]) -> anyhow::Result<Opcode> {
    if line.len() == 2 {
        let value = line[1].parse()?;
        Ok(Opcode::FloatLoad(value))
    } else {
        Err(anyhow::anyhow!("Incorrect number of operands"))
    }
}

fn unprettify_dereference(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Dereference)
}

fn unprettify_consume(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Consume)
}

fn unprettify_complete_assign(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::CompleteAssign)
}

fn unprettify_fix(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Fix)
}

fn unprettify_float(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Float)
}

fn unprettify_add(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Add)
}

fn unprettify_float_add(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatAdd)
}

fn unprettify_subtract(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Subtract)
}

fn unprettify_float_subtract(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatSubtract)
}

fn unprettify_multiply(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Multiply)
}

fn unprettify_float_multiply(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatMultiply)
}

fn unprettify_divide(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Divide)
}

fn unprettify_float_divide(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatDivide)
}

fn unprettify_modulo(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Modulo)
}

fn unprettify_negate(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::IntNegate)
}

fn unprettify_float_negate(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatNegate)
}

fn unprettify_binary_not(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::BinaryNot)
}

fn unprettify_logical_not(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::LogicalNot)
}

fn unprettify_binary_or(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::BinaryOr)
}

fn unprettify_binary_and(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::BinaryAnd)
}

fn unprettify_xor(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Xor)
}

fn unprettify_left_shift(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::LeftShift)
}

fn unprettify_right_shift(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::RightShift)
}

fn unprettify_equal(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Equal)
}

fn unprettify_float_equal(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatEqual)
}

fn unprettify_exlcall(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Exlcall)
}

fn unprettify_not_equal(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::NotEqual)
}

fn unprettify_float_not_equal(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatNotEqual)
}

fn unprettify_nop0x3d(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Nop0x3D)
}

fn unprettify_less_than(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::LessThan)
}

fn unprettify_float_less_than(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatLessThan)
}

fn unprettify_less_than_equal_to(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::LessThanEqualTo)
}

fn unprettify_float_less_than_equal_to(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatLessThanEqualTo)
}

fn unprettify_greater_than(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::GreaterThan)
}

fn unprettify_float_greater_than(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatGreaterThan)
}

fn unprettify_greater_than_equal_to(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::GreaterThanEqualTo)
}

fn unprettify_float_greater_than_equal_to(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::FloatGreaterThanEqualTo)
}

fn unprettify_call_by_id(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, CallById, usize, "call.loc")
}

fn unprettify_call_by_name(line: &[&str]) -> anyhow::Result<Opcode> {
    if line.len() == 3 {
        let name = line[1].to_string();
        let arity = line[2].parse()?;
        Ok(Opcode::CallByName(name, arity))
    } else {
        Err(anyhow::anyhow!("Incorrect number of operands"))
    }
}

fn unprettify_set_return(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::SetReturn)
}

fn unprettify_jump(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_string!(line, Jump)
}

fn unprettify_jump_not_zero(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_string!(line, JumpNotZero)
}

fn unprettify_or(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_string!(line, Or)
}

fn unprettify_jump_zero(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_string!(line, JumpZero)
}

fn unprettify_and(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_string!(line, And)
}

fn unprettify_yield(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Yield)
}

fn unprettify_format(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_unary_int!(line, Format, u8, "printf")
}

fn unprettify_inc(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Inc)
}

fn unprettify_dec(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Dec)
}

fn unprettify_copy(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Copy)
}

fn unprettify_return_false(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::ReturnFalse)
}

fn unprettify_return_true(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::ReturnTrue)
}

fn unprettify_label(line: &[&str]) -> anyhow::Result<Opcode> {
    unprettify_string!(line, Label)
}

fn unprettify_string_equals(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::StringEquals)
}

fn unprettify_string_not_equals(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::StringNotEquals)
}

fn unprettify_return(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Return)
}

fn unprettify_nop0x40(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Nop0x40)
}

fn unprettify_assign(_line: &[&str]) -> anyhow::Result<Opcode> {
    Ok(Opcode::Assign)
}

fn unprettify_opcode(line: &str) -> anyhow::Result<Opcode> {
    let max_parts = if line.starts_with("call ") { 3 } else { 2 };
    let parts: Vec<&str> = line.splitn(max_parts, " ").into_iter().collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty code line."));
    }
    match UNPRETTIFY_DISPATCH_TABLE.get(parts[0]) {
        Some(f) => f(&parts),
        None => return Err(anyhow::anyhow!("Unrecognized opcode '{}'.", parts[0])),
    }
}

pub fn prettify(funcs: &[FunctionData]) -> Vec<PrettyFunctionData> {
    let mut prettified = Vec::new();
    for f in funcs {
        let code: Vec<String> = f.code.iter().map(prettify_opcode).collect();
        prettified.push(PrettyFunctionData {
            function_type: f.function_type,
            arity: f.arity,
            frame_size: f.frame_size,
            name: f.name.clone(),
            args: f.args.clone(),
            code,
            unknown: f.unknown,
            unknown_prefix: f.unknown_prefix.clone(),
            unknown_suffix: f.unknown_suffix.clone(),
        })
    }
    prettified
}

pub fn unprettify(funcs: &[PrettyFunctionData]) -> anyhow::Result<Vec<FunctionData>> {
    let mut unprettified = Vec::new();
    for f in funcs {
        let mut code = Vec::new();
        for i in 0..f.code.len() {
            let line = &f.code[i];
            code.push(unprettify_opcode(line).context(format!(
                "Error in parsing line={} num={} func={:?}",
                line, i, f
            ))?);
        }
        unprettified.push(FunctionData {
            function_type: f.function_type,
            arity: f.arity,
            frame_size: f.frame_size,
            name: f.name.clone(),
            args: f.args.clone(),
            code,
            unknown: f.unknown,
            unknown_prefix: f.unknown_prefix.clone(),
            unknown_suffix: f.unknown_suffix.clone(),
        })
    }
    Ok(unprettified)
}
