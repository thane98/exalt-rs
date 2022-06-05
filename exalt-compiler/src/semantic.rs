use indexmap::IndexMap;

use crate::eval::{evaluate_const_expr, evaluate_enum_access};
use crate::reporting::{CompilerLog, SemanticError, WarningMessage};
use crate::symbol::{SymbolTable, Variable};
use exalt_ast::{
    Annotation, Case, ConstSymbol, DataType, Decl, EnumSymbol, Expr, FunctionSymbol, LabelSymbol,
    Literal, Location, Notation, Operator, Ref, Script, Shared, Stmt, VarSymbol,
};

use exalt_ast::surface::{self, EnumVariant, Identifier};

use std::cell::RefCell;
use std::rc::Rc;

type Result<T> = std::result::Result<T, SemanticError>;

fn make_shared<T>(value: T) -> Rc<RefCell<T>> {
    Rc::new(RefCell::new(value))
}

struct SemanticAnalyzer<'a> {
    symbol_table: SymbolTable,
    log: &'a mut CompilerLog,

    // This is used to track contexts where either a break or continue can be used
    // These must be separate because in certain contexts (ex. match statements) you
    // can use break but not continue
    breaks: usize,
    continues: usize,

    // Tracker for labels in a function
    // We need this because labels can be used before definition
    // and we need to validate that every referenced label is defined somewhere
    labels: Vec<Shared<LabelSymbol>>,

    // Tracker for number of global variables declared
    globals: usize,
}

impl<'a> SemanticAnalyzer<'a> {
    fn new(log: &'a mut CompilerLog) -> Self {
        SemanticAnalyzer {
            symbol_table: SymbolTable::new(),
            log,
            breaks: 0,
            continues: 0,
            labels: Vec::new(),
            globals: 0,
        }
    }

    pub fn analyze(
        log: &mut CompilerLog,
        script: &surface::Script,
    ) -> Option<(Script, SymbolTable)> {
        let mut analyzer = SemanticAnalyzer::new(log);

        // Fill in type definitions and forward declare functions
        analyzer.create_definitions(script);
        // End it here if there are errors since we don't handle bad types/functions gracefully yet.
        if !analyzer.log.has_errors() {
            let script = analyzer.transform_to_ast(script);
            if analyzer.log.has_errors() {
                None
            } else {
                Some((script, analyzer.symbol_table))
            }
        } else {
            None
        }
    }

    fn create_definitions(&mut self, script: &surface::Script) {
        for decl in &script.0 {
            match decl {
                surface::Decl::FunctionAlias {
                    location: _,
                    identifier,
                    alias,
                } => {
                    if let Err(err) = self.symbol_table.define_alias(
                        identifier.value.clone(),
                        alias.value.clone(),
                        identifier.location.clone(),
                    ) {
                        self.log.log_error(err.into());
                    }
                }
                surface::Decl::Constant {
                    location: _,
                    identifier,
                    value,
                } => self.define_constant(identifier, value),
                surface::Decl::Enum {
                    location: _,
                    identifier,
                    variants,
                } => self.define_enum(identifier, variants),
                surface::Decl::Function {
                    location: _,
                    annotations: _,
                    identifier,
                    parameters,
                    body: _,
                } => self.define_simple_function(identifier, parameters),
                surface::Decl::Global(_, identifier, count) => {
                    self.define_global(identifier, count.as_ref())
                }
                _ => {}
            }
        }
    }

    fn define_constant(&mut self, identifier: &Identifier, value: &surface::Expr) {
        match evaluate_const_expr(&self.symbol_table, value) {
            Ok(v) => {
                let symbol =
                    ConstSymbol::new(identifier.value.clone(), identifier.location.clone(), v);
                if let Err(err) = self.symbol_table.define_variable(
                    identifier.value.clone(),
                    Variable::Const(make_shared(symbol)),
                ) {
                    self.log.log_error(err.into());
                }
            }
            Err(err) => self.log.log_error(err.into()),
        }
    }

    fn define_enum(&mut self, ident: &Identifier, variants: &[EnumVariant]) {
        let mut evaluated_variants = IndexMap::new();
        for v in variants {
            match evaluate_const_expr(&self.symbol_table, &v.value) {
                Ok(value) => {
                    if value.data_type() != DataType::Int {
                        self.log.log_error(
                            SemanticError::InvalidType(
                                v.location.clone(),
                                DataType::Int.name(),
                                value.data_type().name(),
                            )
                            .into(),
                        );
                        continue;
                    }

                    let symbol = ConstSymbol::new(
                        v.identifier.value.clone(),
                        v.identifier.location.clone(),
                        value,
                    );
                    if let Some(original) = evaluated_variants.insert(symbol.name.clone(), symbol) {
                        self.log.log_error(
                            SemanticError::SymbolRedefinition(
                                original.location.clone(),
                                v.identifier.location.clone(),
                                v.identifier.value.clone(),
                            )
                            .into(),
                        );
                    }
                }
                Err(err) => self.log.log_error(err.into()),
            }
        }
        let symbol = make_shared(EnumSymbol::new(
            ident.value.clone(),
            ident.location.clone(),
            evaluated_variants,
        ));
        if let Err(err) = self.symbol_table.define_enum(ident.value.clone(), symbol) {
            self.log.log_error(err.into());
        }
    }

    fn define_global(&mut self, identifier: &Identifier, count: Option<&surface::Expr>) {
        let mut symbol =
            VarSymbol::new(identifier.value.clone(), identifier.location.clone(), true);
        symbol.frame_id = Some(self.globals);
        if let Some(count) = count {
            match evaluate_const_expr(&self.symbol_table, count) {
                Ok(length) => {
                    if let Literal::Int(i) = length {
                        if i < 0 {
                            self.log.log_error(
                                SemanticError::NegativeArrayLength(count.location().clone()).into(),
                            );
                        }
                        symbol.array_length = Some(i as usize);
                        self.globals += i as usize;
                    } else {
                        self.log.log_error(
                            SemanticError::InvalidType(
                                count.location().clone(),
                                DataType::Int.name(),
                                length.data_type().name(),
                            )
                            .into(),
                        );
                    }
                }
                Err(err) => self.log.log_error(err.into()),
            }
        } else {
            self.globals += 1;
        }
        let symbol = make_shared(symbol);
        let variable = Variable::Var(symbol);
        if let Err(err) = self
            .symbol_table
            .define_variable(identifier.value.clone(), variable)
        {
            self.log.log_error(err.into());
        }
    }

    fn define_simple_function(&mut self, identifier: &Identifier, params: &[Identifier]) {
        let symbol = make_shared(FunctionSymbol::new(
            identifier.value.clone(),
            identifier.location.clone(),
            params.len(),
            None,
        ));
        if let Err(err) = self
            .symbol_table
            .define_function(identifier.value.clone(), symbol)
        {
            self.log.log_error(err.into());
        }
    }

    fn transform_to_ast(&mut self, script: &surface::Script) -> Script {
        let mut decls = Vec::new();
        for decl in &script.0 {
            self.breaks = 0;
            self.continues = 0;
            self.labels.clear();
            match decl {
                surface::Decl::Function {
                    location: _,
                    annotations,
                    identifier,
                    parameters,
                    body,
                } => {
                    let annotations = self.transform_annotations(annotations);
                    let symbol = self
                        .symbol_table
                        .lookup_function(&identifier.value)
                        .unwrap();
                    self.symbol_table.open_scope();
                    let parameters = self.set_up_function_environment(parameters);
                    let body = match self.evaluate_stmt(body) {
                        Ok(stmt) => stmt,
                        Err(err) => {
                            self.log.log_error(err.into());
                            Stmt::Block(Vec::new())
                        }
                    };
                    self.symbol_table.close_scope();
                    decls.push(Decl::Function {
                        annotations,
                        symbol,
                        parameters,
                        body,
                    })
                }
                surface::Decl::Callback {
                    location: _,
                    annotations,
                    event_type,
                    args,
                    body,
                } => {
                    let annotations = self.transform_annotations(annotations);
                    let event_type = match evaluate_const_expr(&self.symbol_table, event_type) {
                        Ok(v) => match v {
                            Literal::Int(v) => v as usize, // TODO: Warn about narrowing conversion?
                            l => {
                                self.log.log_error(
                                    SemanticError::InvalidType(
                                        event_type.location().clone(),
                                        DataType::Int.name(),
                                        l.data_type().name(),
                                    )
                                    .into(),
                                );
                                0
                            }
                        },
                        Err(err) => {
                            self.log.log_error(err.into());
                            0 // Placeholder since we want to continue evaluating
                        }
                    };
                    let mut evaluated_args = Vec::new();
                    for arg in args {
                        match evaluate_const_expr(&self.symbol_table, arg) {
                            Ok(v) => evaluated_args.push(v),
                            Err(err) => self.log.log_error(err.into()),
                        }
                    }
                    self.symbol_table.open_scope();
                    let body = match self.evaluate_stmt(body) {
                        Ok(stmt) => stmt,
                        Err(err) => {
                            self.log.log_error(err.into());
                            Stmt::Block(Vec::new())
                        }
                    };
                    self.symbol_table.close_scope();
                    decls.push(Decl::Callback {
                        annotations,
                        event_type,
                        args: evaluated_args,
                        body,
                    })
                }
                _ => {}
            }
            self.validate_labels();
        }
        Script::new(decls, self.globals)
    }

    fn set_up_function_environment(&mut self, params: &[Identifier]) -> Vec<Shared<VarSymbol>> {
        let mut parameters = Vec::new();
        for p in params {
            let symbol = make_shared(VarSymbol::new(p.value.clone(), p.location.clone(), false));
            parameters.push(symbol.clone());
            let variable = Variable::Var(symbol);
            if let Err(err) = self.symbol_table.define_variable(p.value.clone(), variable) {
                self.log.log_error(err.into());
            }
        }
        parameters
    }

    fn transform_annotations(&mut self, annotations: &[surface::Annotation]) -> Vec<Annotation> {
        let mut transformed = Vec::new();
        for a in annotations {
            let ident = &a.identifier;
            match ident.value.as_str() {
                "NoDefaultReturn" => {
                    if !a.args.is_empty() {
                        self.log.log_error(
                            SemanticError::SignatureDisagreement(
                                a.args[0].location().clone(),
                                "annotation takes no arguments".to_owned(),
                            )
                            .into(),
                        );
                    } else {
                        transformed.push(Annotation::NoDefaultReturn);
                    }
                }
                "Prefix" => match self.transform_bytes_arguments(&a.args) {
                    Ok(v) => transformed.push(Annotation::Prefix(v)),
                    Err(err) => self.log.log_error(err.into()),
                },
                "Suffix" => match self.transform_bytes_arguments(&a.args) {
                    Ok(v) => transformed.push(Annotation::Suffix(v)),
                    Err(err) => self.log.log_error(err.into()),
                },
                "Unknown" => match self.transform_single_int_argument(&ident.location, &a.args) {
                    Ok(v) => transformed.push(Annotation::Unknown(v)),
                    Err(err) => self.log.log_error(err.into()),
                },
                _ => {
                    self.log
                        .log_error(SemanticError::UndefinedAnnotation(ident.clone()).into());
                }
            }
        }
        transformed
    }

    fn transform_single_int_argument(
        &self,
        location: &Location,
        args: &[surface::Expr],
    ) -> Result<usize> {
        if args.len() != 1 {
            Err(SemanticError::SignatureDisagreement(
                location.clone(),
                "annotation takes a single integer argument".to_owned(),
            ))
        } else {
            let arg = evaluate_const_expr(&self.symbol_table, &args[0])?;
            if let Literal::Int(i) = arg {
                // TODO: Warn about narrowing conversions?
                Ok(i as usize)
            } else {
                Err(SemanticError::SignatureDisagreement(
                    args[0].location().clone(),
                    "annotation takes a single integer argument".to_owned(),
                ))
            }
        }
    }

    fn transform_bytes_arguments(&self, args: &[surface::Expr]) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        for arg in args {
            if let Literal::Int(i) = evaluate_const_expr(&self.symbol_table, arg)? {
                // TODO: Warn about narrowing conversions?
                bytes.push(i as u8);
            } else {
                return Err(SemanticError::SignatureDisagreement(
                    arg.location().clone(),
                    "annotation takes bytes only".to_owned(),
                ));
            }
        }
        Ok(bytes)
    }

    fn validate_labels(&mut self) {
        for label in &self.labels {
            let label = label.borrow();
            if !label.resolved {
                self.log.log_error(
                    SemanticError::UnresolvedLabel(label.location.clone(), label.references.len())
                        .into(),
                );
            } else if label.references.len() == 1 {
                self.log
                    .log_warning(WarningMessage::UnusedLabel(label.location.clone()));
            }
        }
    }

    fn evaluate_stmt(&mut self, stmt: &surface::Stmt) -> Result<Stmt> {
        match stmt {
            surface::Stmt::Assignment {
                location,
                left,
                op,
                right,
            } => self.evaluate_assignment(location, left, *op, right),
            surface::Stmt::Block(_, stmts) => Ok(self.evaluate_block(stmts)),
            surface::Stmt::Break(loc) => {
                if self.breaks == 0 {
                    Err(SemanticError::BadBreak(loc.clone()))
                } else {
                    Ok(Stmt::Break)
                }
            }
            surface::Stmt::Continue(loc) => {
                if self.continues == 0 {
                    Err(SemanticError::BadContinue(loc.clone()))
                } else {
                    Ok(Stmt::Continue)
                }
            }
            surface::Stmt::ExprStmt(_, e) => Ok(Stmt::ExprStmt(self.evaluate_expr(e)?)),
            surface::Stmt::For {
                location: _,
                init,
                check,
                step,
                body,
            } => {
                self.breaks += 1;
                self.continues += 1;
                let result = self.evaluate_for_loop(init, check, step, body);
                self.breaks -= 1;
                self.continues -= 1;
                result
            }
            surface::Stmt::Goto(loc, identifier) => self.evaluate_goto(loc.clone(), identifier),
            surface::Stmt::If {
                location: _,
                condition,
                then_part,
                else_part,
            } => {
                let condition = self.evaluate_expr(condition)?;
                let then_part = Box::new(self.evaluate_stmt(then_part)?);
                let else_part = match else_part {
                    Some(s) => Some(Box::new(self.evaluate_stmt(s)?)),
                    None => None,
                };
                Ok(Stmt::If {
                    condition,
                    then_part,
                    else_part,
                })
            }
            surface::Stmt::Label(loc, identifier) => self.evaluate_label(loc.clone(), identifier),
            surface::Stmt::Match {
                location: _,
                switch,
                cases,
                default,
            } => {
                self.breaks += 1;
                let result = self.evaluate_match(switch, cases, default.as_deref());
                self.breaks -= 1;
                result
            }
            surface::Stmt::Printf(_, args) => {
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.evaluate_expr(arg)?);
                }
                Ok(Stmt::Printf(evaluated_args))
            }
            surface::Stmt::Return(_, e) => {
                if let Some(e) = e {
                    Ok(Stmt::Return(Some(self.evaluate_expr(e)?)))
                } else {
                    Ok(Stmt::Return(None))
                }
            }
            surface::Stmt::VarDecl(_, ident, count) => {
                self.evaluate_var_decl(ident, count.as_ref())
            }
            surface::Stmt::While {
                location: _,
                condition,
                body,
            } => {
                self.breaks += 1;
                self.continues += 1;
                let result = self.evaluate_while_loop(condition, body);
                self.breaks -= 1;
                self.continues -= 1;
                result
            }
            surface::Stmt::Yield(_) => Ok(Stmt::Yield),
        }
    }

    fn evaluate_match(
        &mut self,
        switch: &surface::Expr,
        cases: &[surface::Case],
        default: Option<&surface::Stmt>,
    ) -> Result<Stmt> {
        let switch = self.evaluate_expr(switch)?;
        let mut evaluated_cases = Vec::new();
        for case in cases {
            let mut conditions = Vec::new();
            for cond in &case.conditions {
                conditions.push(self.evaluate_expr(cond)?);
            }
            let body = self.evaluate_stmt(&case.body)?;
            evaluated_cases.push(Case::new(conditions, body));
        }
        let default = match default {
            Some(s) => Some(Box::new(self.evaluate_stmt(s)?)),
            None => None,
        };
        Ok(Stmt::Match {
            switch,
            cases: evaluated_cases,
            default,
        })
    }

    fn evaluate_var_decl(
        &mut self,
        ident: &Identifier,
        count: Option<&surface::Expr>,
    ) -> Result<Stmt> {
        let mut symbol = VarSymbol::new(ident.value.clone(), ident.location.clone(), false);
        let count = if let Some(count) = count {
            let length = evaluate_const_expr(&self.symbol_table, count)?;
            if let Literal::Int(i) = length {
                if i < 0 {
                    return Err(SemanticError::NegativeArrayLength(count.location().clone()));
                }
                symbol.array_length = Some(i as usize);
                Some(i as usize)
            } else {
                return Err(SemanticError::InvalidType(
                    count.location().clone(),
                    DataType::Int.name(),
                    length.data_type().name(),
                ));
            }
        } else {
            None
        };
        let symbol = make_shared(symbol);
        let var = Variable::Var(symbol.clone());
        self.symbol_table
            .define_variable(ident.value.clone(), var)?;
        Ok(Stmt::VarDecl(symbol, count))
    }

    fn evaluate_while_loop(
        &mut self,
        condition: &surface::Expr,
        body: &surface::Stmt,
    ) -> Result<Stmt> {
        let condition = self.evaluate_expr(condition)?;
        let body = Box::new(self.evaluate_stmt(body)?);
        Ok(Stmt::While { condition, body })
    }

    fn evaluate_for_loop(
        &mut self,
        init: &surface::Stmt,
        check: &surface::Expr,
        step: &surface::Stmt,
        body: &surface::Stmt,
    ) -> Result<Stmt> {
        let init = Box::new(self.evaluate_stmt(init)?);
        let check = self.evaluate_expr(check)?;
        let step = Box::new(self.evaluate_stmt(step)?);
        let body = Box::new(self.evaluate_stmt(body)?);
        Ok(Stmt::For {
            init,
            check,
            step,
            body,
        })
    }

    fn evaluate_assignment(
        &mut self,
        location: &Location,
        reference: &surface::Ref,
        op: Operator,
        right: &surface::Expr,
    ) -> Result<Stmt> {
        if op == Operator::Assign && matches!(reference, surface::Ref::Var(_)) {
            self.declare_variable(
                location,
                reference,
                match right {
                    surface::Expr::Array(_, elements) => Some(elements.len()),
                    _ => None,
                },
            )?;
        }
        let expr = self.evaluate_reference(reference)?;
        match expr {
            Expr::Ref(reference) => Ok(Stmt::Assignment {
                left: reference,
                op,
                right: self.evaluate_expr(right)?,
            }),
            _ => Err(SemanticError::ExpectedReferenceOperand(location.clone())),
        }
    }

    fn declare_variable(
        &mut self,
        location: &Location,
        reference: &surface::Ref,
        array_length: Option<usize>,
    ) -> Result<Shared<VarSymbol>> {
        let id = match reference {
            surface::Ref::Var(id) => id,
            _ => panic!(),
        };
        if let Some(symbol) = self.symbol_table.lookup_variable(&id.value) {
            if let Variable::Var(symbol) = symbol {
                Ok(symbol)
            } else {
                Err(SemanticError::ExpectedReferenceOperand(location.clone()))
            }
        } else {
            let mut symbol = VarSymbol::new(id.value.clone(), id.location.clone(), false);
            symbol.array_length = array_length;
            let symbol = make_shared(symbol);
            self.symbol_table
                .define_variable(id.value.clone(), Variable::Var(symbol.clone()))?;
            Ok(symbol)
        }
    }

    // Why doesn't this return Result<Stmt>?
    // We already determine whether the run failed based on the presence of errors in the compiler log
    // We also want to catch as many errors as we can in a single compiler run
    // Thus, this builds a block from successful statements and logs failures so we can evaluate
    // as much of the script as possible
    fn evaluate_block(&mut self, stmts: &[surface::Stmt]) -> Stmt {
        let mut evaluated = Vec::new();
        for stmt in stmts {
            match self.evaluate_stmt(stmt) {
                Ok(s) => evaluated.push(s),
                Err(err) => self.log.log_error(err.into()),
            }
        }
        Stmt::Block(evaluated)
    }

    fn evaluate_goto(&mut self, location: Location, identifier: &Identifier) -> Result<Stmt> {
        let symbol = if let Some(symbol) = self.symbol_table.lookup_label(&identifier.value) {
            symbol.borrow_mut().references.push(location);
            symbol
        } else {
            let symbol = make_shared(LabelSymbol::new(
                identifier.value.clone(),
                identifier.location.clone(),
                vec![location],
                false,
            ));
            self.symbol_table
                .define_label(identifier.value.clone(), symbol.clone())
                .unwrap();
            self.labels.push(symbol.clone());
            symbol
        };
        Ok(Stmt::Goto(symbol))
    }

    fn evaluate_label(&mut self, location: Location, identifier: &Identifier) -> Result<Stmt> {
        let symbol = if let Some(symbol) = self.symbol_table.lookup_label(&identifier.value) {
            if symbol.borrow().resolved {
                Err(SemanticError::SymbolRedefinition(
                    symbol.borrow().location.clone(),
                    identifier.location.clone(),
                    identifier.value.clone(),
                ))
            } else {
                {
                    let mut symbol = symbol.borrow_mut();
                    symbol.resolved = true;
                    symbol.references.push(location);
                    symbol.location = identifier.location.clone();
                }
                Ok(symbol)
            }
        } else {
            let symbol = make_shared(LabelSymbol::new(
                identifier.value.clone(),
                identifier.location.clone(),
                vec![location],
                true,
            ));
            self.symbol_table
                .define_label(identifier.value.clone(), symbol.clone())
                .unwrap();
            self.labels.push(symbol.clone());
            Ok(symbol)
        }?;
        Ok(Stmt::Label(symbol))
    }

    fn evaluate_expr(&mut self, expr: &surface::Expr) -> Result<Expr> {
        match expr {
            surface::Expr::Array(_, values) => {
                let mut evaluated = Vec::new();
                for v in values {
                    evaluated.push(self.evaluate_expr(v)?);
                }
                Ok(Expr::Array(evaluated))
            }
            surface::Expr::Literal(_, literal) => Ok(Expr::Literal(literal.clone())),
            surface::Expr::EnumAccess(_, name, variant) => Ok(Expr::Literal(evaluate_enum_access(
                &self.symbol_table,
                name,
                variant,
            )?)),
            surface::Expr::Unary(_, operand, op) => {
                if let surface::Expr::Literal(_, _) = operand.as_ref() {
                    if *op != Operator::FloatNegate {
                        Ok(Expr::Literal(evaluate_const_expr(
                            &self.symbol_table,
                            expr,
                        )?))
                    } else {
                        Ok(Expr::Unary(*op, Box::new(self.evaluate_expr(operand)?)))
                    }
                } else {
                    Ok(Expr::Unary(*op, Box::new(self.evaluate_expr(operand)?)))
                }
            }
            surface::Expr::Binary(_, left, op, right) => {
                // Check if we can try constant folding
                if let (surface::Expr::Literal(..), surface::Expr::Literal(..)) =
                    (left.as_ref(), right.as_ref())
                {
                    Ok(Expr::Literal(evaluate_const_expr(
                        &self.symbol_table,
                        expr,
                    )?))
                } else {
                    let left = self.evaluate_expr(left)?;
                    let right = self.evaluate_expr(right)?;
                    Ok(Expr::Binary(Box::new(left), *op, Box::new(right)))
                }
            }
            surface::Expr::FunctionCall(_, identifier, args) => {
                self.evaluate_function_call(identifier, args)
            }
            surface::Expr::Ref(_, reference) => self.evaluate_reference(reference),
            surface::Expr::Grouped(_, e) => self.evaluate_expr(e),
            surface::Expr::Increment(location, reference, op, notation) => {
                self.evaluate_increment(location, reference, *op, *notation)
            }
            surface::Expr::AddressOf(location, reference) => {
                self.evaluate_address_of(location, reference)
            }
        }
    }

    fn evaluate_function_call(
        &mut self,
        ident: &Identifier,
        args: &[surface::Expr],
    ) -> Result<Expr> {
        let symbol = if let Some(symbol) = self.symbol_table.lookup_function(&ident.value) {
            symbol
        } else {
            let symbol = make_shared(FunctionSymbol::new(
                ident.value.clone(),
                ident.location.clone(),
                args.len(),
                None,
            ));
            self.symbol_table
                .define_function(ident.value.clone(), symbol.clone())?;
            symbol
        };
        let mut evaluated_args = Vec::new();
        {
            let expected_length = symbol.borrow().arity;
            if args.len() != expected_length {
                return Err(SemanticError::BadArgCount(
                    ident.location.clone(),
                    expected_length,
                    args.len(),
                ));
            }
            for arg in args {
                evaluated_args.push(self.evaluate_expr(arg)?);
            }
        }

        Ok(Expr::FunctionCall(symbol, evaluated_args))
    }

    fn evaluate_reference(&mut self, reference: &surface::Ref) -> Result<Expr> {
        match reference {
            surface::Ref::Var(identifier) => match self.find_var(identifier)? {
                Variable::Const(c) => Ok(Expr::Literal(c.borrow().value.clone())),
                Variable::Var(symbol) => Ok(Expr::Ref(Ref::Var(symbol))),
            },
            surface::Ref::Index(identifier, index) => match self.find_var(identifier)? {
                Variable::Const(_) => Err(SemanticError::ExpectedReferenceOperand(
                    identifier.location.clone(),
                )),
                Variable::Var(symbol) => {
                    let evaluated_index = self.evaluate_expr(index)?;
                    Ok(Expr::Ref(Ref::Index(symbol, Box::new(evaluated_index))))
                }
            },
            surface::Ref::Dereference(identifier, index) => match self.find_var(identifier)? {
                Variable::Const(_) => Err(SemanticError::ExpectedReferenceOperand(
                    identifier.location.clone(),
                )),
                Variable::Var(symbol) => {
                    let evaluated_index = if let Some(index) = index {
                        Some(Box::new(self.evaluate_expr(index)?))
                    } else {
                        None
                    };
                    Ok(Expr::Ref(Ref::Dereference(symbol, evaluated_index)))
                }
            },
        }
    }

    fn find_var(&self, identifier: &Identifier) -> Result<Variable> {
        self.symbol_table
            .lookup_variable(&identifier.value)
            .ok_or_else(|| SemanticError::UndefinedVariable(identifier.clone()))
    }

    fn evaluate_increment(
        &mut self,
        location: &Location,
        reference: &surface::Ref,
        op: Operator,
        notation: Notation,
    ) -> Result<Expr> {
        if let Expr::Ref(reference) = self.evaluate_reference(reference)? {
            Ok(Expr::Increment(reference, op, notation))
        } else {
            Err(SemanticError::ExpectedReferenceOperand(location.clone()))
        }
    }

    fn evaluate_address_of(
        &mut self,
        location: &Location,
        reference: &surface::Ref,
    ) -> Result<Expr> {
        if let Expr::Ref(reference) = self.evaluate_reference(reference)? {
            Ok(Expr::AddressOf(reference))
        } else {
            Err(SemanticError::ExpectedReferenceOperand(location.clone()))
        }
    }
}

pub fn analyze(script: &surface::Script, log: &mut CompilerLog) -> Option<(Script, SymbolTable)> {
    SemanticAnalyzer::analyze(log, script)
}
