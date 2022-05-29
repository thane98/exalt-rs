use std::collections::{HashMap, HashSet};

use exalt_ast::{
    Annotation, ArrayInit, Decl, Expr, Literal, Notation, Operator, Ref, Script, Stmt,
};
use exalt_lir::{CallbackArg, Game, Opcode, RawScript};

use thiserror::Error;

type RawFunction = exalt_lir::Function;
type Result<T> = std::result::Result<T, CodeGenerationError>;

#[derive(Debug, Error)]
pub enum CodeGenerationError {
    #[error("break or continue without a target")]
    BadBreakOrContinue,

    #[error("{0}")]
    BadAssembly(String),
}

#[derive(Debug)]
struct FunctionGenerationConfig {
    default_return: bool,
    unknown_value: u8,
    suffix: Vec<u8>,
    prefix: Vec<u8>,
}

impl Default for FunctionGenerationConfig {
    fn default() -> Self {
        Self {
            default_return: true,
            unknown_value: 0,
            suffix: Vec::new(),
            prefix: Vec::new(),
        }
    }
}

fn to_opcode(op: Operator) -> Opcode {
    match op {
        Operator::Divide | Operator::AssignDivide => Opcode::Divide,
        Operator::FloatDivide => Opcode::FloatDivide,
        Operator::Multiply | Operator::AssignMultiply => Opcode::Multiply,
        Operator::FloatMultiply => Opcode::FloatMultiply,
        Operator::Modulo | Operator::AssignModulo => Opcode::Modulo,
        Operator::Add | Operator::AssignAdd => Opcode::Add,
        Operator::FloatAdd => Opcode::FloatAdd,
        Operator::Subtract | Operator::AssignSubtract => Opcode::Subtract,
        Operator::FloatSubtract => Opcode::FloatSubtract,
        Operator::LeftShift | Operator::AssignLeftShift => Opcode::LeftShift,
        Operator::RightShift | Operator::AssignRightShift => Opcode::RightShift,
        Operator::LessThan => Opcode::LessThan,
        Operator::FloatLessThan => Opcode::FloatLessThan,
        Operator::LessThanEqualTo => Opcode::LessThanEqualTo,
        Operator::FloatLessThanEqualTo => Opcode::FloatLessThanEqualTo,
        Operator::GreaterThan => Opcode::GreaterThan,
        Operator::FloatGreaterThan => Opcode::FloatGreaterThan,
        Operator::GreaterThanEqualTo => Opcode::GreaterThanEqualTo,
        Operator::FloatGreaterThanEqualTo => Opcode::FloatGreaterThanEqualTo,
        Operator::Equal => Opcode::Equal,
        Operator::FloatEqual => Opcode::FloatEqual,
        Operator::NotEqual => Opcode::NotEqual,
        Operator::FloatNotEqual => Opcode::FloatNotEqual,
        Operator::BitwiseAnd | Operator::AssignBitwiseAnd => Opcode::BinaryAnd,
        Operator::Xor | Operator::AssignXor => Opcode::Xor,
        Operator::BitwiseOr | Operator::AssignBitwiseOr => Opcode::BinaryOr,
        Operator::LogicalAnd => unimplemented!(),
        Operator::LogicalOr => unimplemented!(),
        Operator::LogicalNot => Opcode::LogicalNot,
        Operator::BitwiseNot => Opcode::BinaryNot,
        Operator::Negate => Opcode::IntNegate,
        Operator::FloatNegate => Opcode::FloatNegate,
        Operator::Increment => Opcode::Inc,
        Operator::Decrement => Opcode::Dec,
        Operator::Assign => Opcode::Assign,
    }
}

/// Ugly way of determining whether to write l-value or r-value opcodes
enum ValueCategory {
    LValue,
    RValue,
}

struct CodeGenerator {
    function_to_call_id: HashMap<String, usize>,
    next_label: usize,
    frame_size: usize,
    continue_labels: Vec<String>,
    break_labels: Vec<String>,
    assigned_variables: HashSet<String>,
}

impl CodeGenerator {
    pub fn serialize(script: &Script) -> Result<RawScript> {
        let mut functions = Vec::new();
        let mut generator = CodeGenerator {
            function_to_call_id: CodeGenerator::generate_function_to_call_id(script),
            next_label: 0,
            frame_size: 0,
            continue_labels: Vec::new(),
            break_labels: Vec::new(),
            assigned_variables: HashSet::new(),
        };
        for decl in &script.decls {
            functions.push(generator.generate_function_data(decl)?);
        }
        Ok(RawScript {
            functions,
            global_frame_size: script.globals,
        })
    }

    fn generate_function_to_call_id(script: &Script) -> HashMap<String, usize> {
        let mut entries = HashMap::new();
        for (call_id, decl) in script.decls.iter().enumerate() {
            if let Decl::Function {
                annotations: _,
                symbol,
                parameters: _,
                body: _,
            } = decl
            {
                let name = symbol.borrow().name.clone();
                entries.insert(name, call_id);
            }
        }
        entries
    }

    fn generate_function_data(&mut self, decl: &Decl) -> Result<RawFunction> {
        self.frame_size = 0;
        self.continue_labels.clear();
        self.break_labels.clear();
        self.assigned_variables.clear();

        match decl {
            Decl::Function {
                annotations,
                symbol,
                parameters,
                body,
            } => {
                let config = CodeGenerator::annotations_to_config(annotations);
                for (i, p) in parameters.iter().enumerate() {
                    p.borrow_mut().frame_id = Some(i);
                }
                self.frame_size += parameters.len();
                let mut code = Vec::new();
                self.convert_stmt_to_opcodes(&mut code, body)?;
                if config.default_return {
                    code.push(Opcode::ReturnFalse);
                }
                let symbol = symbol.borrow();
                Ok(RawFunction {
                    event: 0,
                    arity: parameters.len() as u8,
                    frame_size: self.frame_size,
                    unknown: config.unknown_value,
                    prefix: config.prefix,
                    suffix: config.suffix,
                    name: if symbol.name.contains("::") {
                        Some(symbol.name.clone())
                    } else {
                        None
                    },
                    args: Vec::new(),
                    code,
                })
            }
            Decl::Callback {
                annotations,
                event_type,
                args,
                body,
            } => {
                let config = CodeGenerator::annotations_to_config(annotations);
                let mut event_args = Vec::new();
                for arg in args {
                    event_args.push(match arg {
                        Literal::Int(v) => CallbackArg::Int(*v),
                        Literal::Str(v) => CallbackArg::Str(v.clone()),
                        Literal::Float(v) => CallbackArg::Float(*v),
                    })
                }
                let mut code = Vec::new();
                self.convert_stmt_to_opcodes(&mut code, body)?;
                if config.default_return {
                    code.push(Opcode::ReturnFalse);
                }
                Ok(RawFunction {
                    event: *event_type as u8,
                    arity: args.len() as u8,
                    frame_size: self.frame_size,
                    unknown: config.unknown_value,
                    prefix: config.prefix,
                    suffix: config.suffix,
                    name: None,
                    args: event_args,
                    code,
                })
            }
        }
    }

    fn annotations_to_config(annotations: &[Annotation]) -> FunctionGenerationConfig {
        let mut config = FunctionGenerationConfig::default();
        for a in annotations {
            match a {
                Annotation::NoDefaultReturn => config.default_return = false,
                Annotation::Prefix(v) => config.prefix = v.clone(),
                Annotation::Suffix(v) => config.suffix = v.clone(),
                Annotation::Unknown(v) => config.unknown_value = *v as u8,
            }
        }
        config
    }

    fn generate_label(&mut self) -> String {
        let label = format!("___exalt__autogenerated__label___{}", self.next_label);
        self.next_label += 1;
        label
    }

    fn convert_stmt_to_opcodes(&mut self, opcodes: &mut Vec<Opcode>, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Assignment { left, op, right } => {
                if let Operator::Assign = op {
                    let frame_id = self.process_assignment_lhs(left, right)?;
                    match right {
                        Expr::Array(ArrayInit::Empty(_)) => {}
                        Expr::Array(ArrayInit::Static(values)) => {
                            for (i, value) in values.iter().enumerate() {
                                opcodes.push(Opcode::VarAddr((frame_id + i) as u16));
                                self.convert_expr_to_opcodes(opcodes, value)?;
                                opcodes.push(Opcode::Assign);
                            }
                        }
                        _ => {
                            self.convert_ref_to_opcodes(opcodes, left, ValueCategory::LValue)?;
                            self.convert_expr_to_opcodes(opcodes, right)?;
                            opcodes.push(Opcode::Assign);
                        }
                    }

                    Ok(())
                } else {
                    let op = op.unwrap_shorthand().unwrap();
                    self.convert_ref_to_opcodes(opcodes, left, ValueCategory::LValue)?;
                    opcodes.push(Opcode::Dereference);
                    self.convert_expr_to_opcodes(opcodes, right)?;
                    opcodes.push(to_opcode(op));
                    opcodes.push(Opcode::Assign);
                    Ok(())
                }
            }
            Stmt::Block(stmts) => {
                for stmt in stmts {
                    self.convert_stmt_to_opcodes(opcodes, stmt)?;
                }
                Ok(())
            }
            Stmt::Break => {
                if let Some(l) = self.break_labels.last() {
                    opcodes.push(Opcode::Jump(l.clone()));
                    Ok(())
                } else {
                    Err(CodeGenerationError::BadBreakOrContinue)
                }
            }
            Stmt::Continue => {
                if let Some(l) = self.continue_labels.last() {
                    opcodes.push(Opcode::Jump(l.clone()));
                    Ok(())
                } else {
                    Err(CodeGenerationError::BadBreakOrContinue)
                }
            }
            Stmt::ExprStmt(e) => {
                self.convert_expr_to_opcodes(opcodes, e)?;
                opcodes.push(Opcode::Consume);
                Ok(())
            }
            Stmt::For {
                init,
                check,
                step,
                body,
            } => {
                let step_label = self.generate_label();
                let check_label = self.generate_label();
                let done_label = self.generate_label();
                self.continue_labels.push(step_label.clone());
                self.break_labels.push(done_label.clone());
                self.convert_stmt_to_opcodes(opcodes, init)?;
                opcodes.push(Opcode::Jump(check_label.clone()));
                opcodes.push(Opcode::Label(step_label.clone()));
                self.convert_stmt_to_opcodes(opcodes, step)?;
                opcodes.push(Opcode::Label(check_label));
                self.convert_expr_to_opcodes(opcodes, check)?;
                opcodes.push(Opcode::JumpZero(done_label.clone()));
                self.convert_stmt_to_opcodes(opcodes, body)?;
                opcodes.push(Opcode::Jump(step_label));
                opcodes.push(Opcode::Label(done_label));
                self.continue_labels.pop();
                self.break_labels.pop();
                Ok(())
            }
            Stmt::Goto(symbol) => {
                opcodes.push(Opcode::Jump(symbol.borrow().name.clone()));
                Ok(())
            }
            Stmt::If {
                condition,
                then_part,
                else_part,
            } => {
                self.convert_expr_to_opcodes(opcodes, condition)?;
                let done_label = self.generate_label();
                let else_label = if else_part.is_some() {
                    Some(self.generate_label())
                } else {
                    None
                };
                opcodes.push(Opcode::JumpZero(if else_part.is_none() {
                    done_label.clone()
                } else {
                    else_label.clone().unwrap()
                }));
                self.convert_stmt_to_opcodes(opcodes, then_part)?;
                if let Some(p) = else_part {
                    opcodes.push(Opcode::Jump(done_label.clone()));
                    opcodes.push(Opcode::Label(else_label.unwrap()));
                    self.convert_stmt_to_opcodes(opcodes, p)?;
                }
                opcodes.push(Opcode::Label(done_label));
                Ok(())
            }
            Stmt::Label(symbol) => {
                opcodes.push(Opcode::Label(symbol.borrow().name.clone()));
                Ok(())
            }
            Stmt::Match {
                switch,
                cases,
                default,
            } => {
                self.convert_expr_to_opcodes(opcodes, switch)?;
                let done_label = self.generate_label();
                self.break_labels.push(done_label.clone());
                let mut next_case_label = self.generate_label();
                for (i, case) in cases.iter().enumerate() {
                    opcodes.push(Opcode::Label(next_case_label));
                    let block_label = self.generate_label();
                    next_case_label = if i == cases.len() - 1 && default.is_none() {
                        done_label.clone()
                    } else {
                        self.generate_label()
                    };
                    for condition in &case.conditions {
                        opcodes.push(Opcode::Copy);
                        self.convert_expr_to_opcodes(opcodes, condition)?;
                        opcodes.push(Opcode::Equal);
                        opcodes.push(Opcode::JumpNotZero(block_label.clone()));
                    }
                    opcodes.push(Opcode::Jump(next_case_label.clone()));
                    opcodes.push(Opcode::Label(block_label.clone()));
                    self.convert_stmt_to_opcodes(opcodes, &case.body)?;
                    opcodes.push(Opcode::Jump(done_label.clone()));
                }
                if let Some(stmt) = default {
                    opcodes.push(Opcode::Label(next_case_label));
                    self.convert_stmt_to_opcodes(opcodes, stmt)?;
                    opcodes.push(Opcode::Jump(done_label.clone()));
                }
                opcodes.push(Opcode::Label(done_label));
                opcodes.push(Opcode::Consume);
                self.break_labels.pop();
                Ok(())
            }
            Stmt::Printf(args) => {
                for arg in args {
                    self.convert_expr_to_opcodes(opcodes, arg)?;
                }
                opcodes.push(Opcode::Format(args.len() as u8));
                Ok(())
            }
            Stmt::Return(v) => match v {
                Some(v) => {
                    match v {
                        Expr::Literal(Literal::Int(i)) => {
                            if *i == 0 {
                                opcodes.push(Opcode::ReturnFalse);
                            } else if *i == 1 {
                                opcodes.push(Opcode::ReturnTrue);
                            } else {
                                self.convert_expr_to_opcodes(opcodes, v)?;
                                opcodes.push(Opcode::Return);
                            }
                        }
                        _ => {
                            self.convert_expr_to_opcodes(opcodes, v)?;
                            opcodes.push(Opcode::Return);
                        }
                    }
                    Ok(())
                }
                None => {
                    // TODO: FE9 doesn't support this opcode, so it should push 0 then return
                    opcodes.push(Opcode::ReturnFalse);
                    Ok(())
                }
            },
            Stmt::VarDecl(symbol) => {
                symbol.borrow_mut().frame_id = Some(self.frame_size);
                self.frame_size += 1;
                Ok(())
            }
            Stmt::While { condition, body } => {
                let check_label = self.generate_label();
                let done_label = self.generate_label();
                self.continue_labels.push(check_label.clone());
                self.break_labels.push(done_label.clone());
                opcodes.push(Opcode::Label(check_label.clone()));
                self.convert_expr_to_opcodes(opcodes, condition)?;
                opcodes.push(Opcode::JumpZero(done_label.clone()));
                self.convert_stmt_to_opcodes(opcodes, body)?;
                opcodes.push(Opcode::Jump(check_label));
                opcodes.push(Opcode::Label(done_label));
                self.continue_labels.pop();
                self.break_labels.pop();
                Ok(())
            }
            Stmt::Yield => {
                opcodes.push(Opcode::Yield);
                Ok(())
            }
        }
    }

    fn process_assignment_lhs(&mut self, reference: &Ref, right: &Expr) -> Result<usize> {
        let mut symbol = match reference {
            Ref::Var(symbol) => symbol,
            Ref::Index(symbol, _) => symbol,
            Ref::Dereference(symbol, _) => symbol,
        }
        .borrow_mut();
        if !self.assigned_variables.contains(&symbol.name) {
            if symbol.frame_id.is_none() {
                symbol.frame_id = Some(self.frame_size);
                match right {
                    Expr::Array(ArrayInit::Empty(count)) => self.frame_size += count,
                    Expr::Array(ArrayInit::Static(elements)) => { 
                        self.frame_size += elements.len();
                    },
                    _ => self.frame_size += 1,
                }
            }
            self.assigned_variables.insert(symbol.name.clone());
        }
        
        Ok(symbol.frame_id.unwrap())
    }

    fn convert_expr_to_opcodes(&mut self, opcodes: &mut Vec<Opcode>, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Array(_) => unimplemented!(),
            Expr::Literal(l) => {
                opcodes.push(match l {
                    Literal::Int(i) => Opcode::IntLoad(*i),
                    Literal::Str(s) => Opcode::StrLoad(s.clone()),
                    Literal::Float(f) => Opcode::FloatLoad(*f),
                });
                Ok(())
            }
            Expr::Grouped(e) => self.convert_expr_to_opcodes(opcodes, e),
            Expr::Unary(op, expr) => {
                self.convert_expr_to_opcodes(opcodes, expr)?;
                opcodes.push(to_opcode(*op));
                Ok(())
            }
            Expr::Binary(left, op, right) => {
                self.convert_expr_to_opcodes(opcodes, left)?;
                if *op == Operator::LogicalAnd || *op == Operator::LogicalOr {
                    let end_label = self.generate_label();
                    if let Operator::LogicalAnd = *op {
                        opcodes.push(Opcode::And(end_label.clone()));
                    } else {
                        opcodes.push(Opcode::Or(end_label.clone()));
                    }
                    self.convert_expr_to_opcodes(opcodes, right)?;
                    opcodes.push(Opcode::Label(end_label));
                } else {
                    self.convert_expr_to_opcodes(opcodes, right)?;
                    opcodes.push(to_opcode(*op));
                }
                Ok(())
            }
            Expr::FunctionCall(symbol, args) => {
                let symbol = symbol.borrow();
                for arg in args {
                    self.convert_expr_to_opcodes(opcodes, arg)?;
                }
                match symbol.name.as_str() {
                    "negate" => opcodes.push(Opcode::IntNegate),
                    "fix" => opcodes.push(Opcode::Fix),
                    "float" => opcodes.push(Opcode::Float),
                    "streq" => opcodes.push(Opcode::StringEquals),
                    "strne" => opcodes.push(Opcode::StringNotEquals),
                    _ => match self.function_to_call_id.get(&symbol.name) {
                        Some(id) => {
                            opcodes.push(Opcode::CallById(*id));
                        }
                        None => {
                            opcodes
                                .push(Opcode::CallByName(symbol.name.clone(), symbol.arity as u8));
                        }
                    },
                }
                Ok(())
            }
            Expr::Ref(r) => self.convert_ref_to_opcodes(opcodes, r, ValueCategory::RValue),
            Expr::Increment(expr, op, notation) => {
                match notation {
                    Notation::Prefix => {
                        self.convert_ref_to_opcodes(opcodes, expr, ValueCategory::LValue)?;
                        opcodes.push(to_opcode(*op));
                        self.convert_ref_to_opcodes(opcodes, expr, ValueCategory::RValue)?;
                    }
                    Notation::Postfix => {
                        self.convert_ref_to_opcodes(opcodes, expr, ValueCategory::RValue)?;
                        self.convert_ref_to_opcodes(opcodes, expr, ValueCategory::LValue)?;
                        opcodes.push(to_opcode(*op));
                    }
                }
                Ok(())
            }
            Expr::AddressOf(r) => self.convert_ref_to_opcodes(opcodes, r, ValueCategory::LValue),
        }
    }

    fn convert_ref_to_opcodes(
        &mut self,
        opcodes: &mut Vec<Opcode>,
        reference: &Ref,
        category: ValueCategory,
    ) -> Result<()> {
        match reference {
            Ref::Var(symbol) => {
                let frame_id = symbol.borrow().frame_id.unwrap() as u16;
                let global = symbol.borrow().global;
                match category {
                    ValueCategory::LValue => opcodes.push(if global {
                        Opcode::GlobalVarAddr(frame_id)
                    } else {
                        Opcode::VarAddr(frame_id)
                    }),
                    ValueCategory::RValue => opcodes.push(if global {
                        Opcode::GlobalVarLoad(frame_id)
                    } else {
                        Opcode::VarLoad(frame_id)
                    }),
                }
                Ok(())
            }
            Ref::Index(symbol, index) => {
                self.convert_expr_to_opcodes(opcodes, index)?;
                let frame_id = symbol.borrow().frame_id.unwrap() as u16;
                let global = symbol.borrow().global;
                match category {
                    ValueCategory::LValue => opcodes.push(if global {
                        Opcode::GlobalArrAddr(frame_id)
                    } else {
                        Opcode::ArrAddr(frame_id)
                    }),
                    ValueCategory::RValue => opcodes.push(if global {
                        Opcode::GlobalArrLoad(frame_id)
                    } else {
                        Opcode::ArrLoad(frame_id)
                    }),
                }
                Ok(())
            }
            Ref::Dereference(symbol, index) => {
                if let Some(expr) = index {
                    self.convert_expr_to_opcodes(opcodes, expr)?;
                } else {
                    opcodes.push(Opcode::IntLoad(0));
                }
                let frame_id = symbol.borrow().frame_id.unwrap() as u16;
                let global = symbol.borrow().global;
                match category {
                    ValueCategory::LValue => opcodes.push(if global {
                        Opcode::GlobalPtrAddr(frame_id)
                    } else {
                        Opcode::PtrAddr(frame_id)
                    }),
                    ValueCategory::RValue => opcodes.push(if global {
                        Opcode::GlobalPtrLoad(frame_id)
                    } else {
                        Opcode::PtrLoad(frame_id)
                    }),
                }
                Ok(())
            }
        }
    }
}

pub fn serialize(script_name: &str, script: &Script, game: Game) -> Result<Vec<u8>> {
    let script_binary = CodeGenerator::serialize(script)?;
    let raw = exalt_assembler::assemble(&script_binary, script_name, game)
        .map_err(|err| CodeGenerationError::BadAssembly(format!("{:?}", err)))?;
    Ok(raw)
}
