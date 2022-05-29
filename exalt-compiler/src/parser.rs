use crate::lexer::{Peekable, Token};
use crate::reporting::{CompilerLog, ParserError};
use exalt_ast::surface::{
    Annotation, ArrayInit, Case, Decl, EnumVariant, Expr, Identifier, Ref, Script, Stmt,
};
use exalt_ast::{FileId, Literal, Location, Notation, Operator, Precedence};

type Result<T> = std::result::Result<T, ParserError>;

/// The Exalt parser
struct Parser<'a, 'source: 'a> {
    file_id: FileId,
    lex: Peekable<'source>,
    log: &'a mut CompilerLog<'source>,
}

impl<'a, 'source> Parser<'a, 'source> {
    pub fn new(file_id: FileId, source: &'source str, log: &'a mut CompilerLog<'source>) -> Self {
        Parser {
            file_id,
            lex: Peekable::new(source),
            log,
        }
    }

    pub fn parse_script(&mut self) -> Script {
        let mut decls = Vec::new();
        while !self.at_end() {
            match self.parse_decl() {
                Ok(d) => decls.push(d),
                Err(e) => {
                    self.log.log_error(e.into());
                    self.skip_to_next_decl();
                }
            }
        }
        Script::new(decls)
    }

    fn skip_to_next_decl(&mut self) {
        while let Some(t) = self.lex.peek() {
            match t {
                Token::Callback | Token::Def | Token::Enum | Token::Const => break,
                _ => {
                    self.lex.next();
                }
            }
        }
    }

    fn at_end(&mut self) -> bool {
        self.lex.peek().is_none()
    }

    fn parse_decl(&mut self) -> Result<Decl> {
        match self.peek_token()? {
            Token::Const => self.parse_const(),
            Token::Enum => self.parse_enum(),
            Token::Let => self.parse_global(),
            Token::AtSign | Token::Def | Token::Callback => {
                let annotations = self.parse_annotations()?;
                match self.peek_token()? {
                    Token::Def => self.parse_function(annotations),
                    Token::Callback => self.parse_callback(annotations),
                    _ => Err(ParserError::ExpectedDecl(self.location())),
                }
            }
            _ => Err(ParserError::ExpectedDecl(self.location())),
        }
    }

    fn parse_annotations(&mut self) -> Result<Vec<Annotation>> {
        let mut annotations = Vec::new();
        while let Token::AtSign = self.peek_token()? {
            annotations.push(self.parse_annotation()?);
        }
        Ok(annotations)
    }

    fn parse_annotation(&mut self) -> Result<Annotation> {
        self.consume(Token::AtSign)?;
        let loc = self.location();
        let identifier = self.parse_identifier()?;
        let args = if let Token::LeftParen = self.peek_token()? {
            self.consume(Token::LeftParen)?;
            let args = if let Token::RightParen = self.peek_token()? {
                Vec::new()
            } else {
                self.parse_comma_separated_expressions(Token::RightParen)?
            };
            self.consume(Token::RightParen)?;
            args
        } else {
            Vec::new()
        };
        Ok(Annotation::new(
            self.location().merge(&loc),
            identifier,
            args,
        ))
    }

    fn parse_const(&mut self) -> Result<Decl> {
        self.consume(Token::Const)?;
        let loc = self.location();
        let identifier = self.parse_identifier()?;
        self.consume(Token::Assign)?;
        let value = self.parse_expression(Precedence::Lowest)?;
        self.consume(Token::Semicolon)?;
        Ok(Decl::Constant {
            location: self.location().merge(&loc),
            identifier,
            value,
        })
    }

    fn parse_enum(&mut self) -> Result<Decl> {
        self.consume(Token::Enum)?;
        let loc = self.location();
        let identifier = self.parse_identifier()?;
        self.consume(Token::LeftBrace)?;
        let mut variants = Vec::new();
        if self.peek_token()? != Token::RightBrace {
            variants.push(self.parse_enum_variant()?);
            while self.peek_token()? == Token::Comma {
                self.consume(Token::Comma)?;
                if self.peek_token()? != Token::RightBrace {
                    variants.push(self.parse_enum_variant()?);
                }
            }
        }
        self.consume(Token::RightBrace)?;
        Ok(Decl::Enum {
            location: self.location().merge(&loc),
            identifier,
            variants,
        })
    }

    fn parse_enum_variant(&mut self) -> Result<EnumVariant> {
        let identifier = self.parse_identifier()?;
        let loc = self.location();
        self.consume(Token::Assign)?;
        let value = self.parse_expression(Precedence::Lowest)?;
        Ok(EnumVariant::new(
            self.location().merge(&loc),
            identifier,
            value,
        ))
    }

    fn parse_function(&mut self, annotations: Vec<Annotation>) -> Result<Decl> {
        self.consume(Token::Def)?;
        let loc = self.location();
        let identifier = self.parse_identifier()?;
        let parameters = self.parse_function_parameters()?;
        let signature_location = self.location().merge(&loc);
        let body = self.parse_block()?;
        Ok(Decl::Function {
            location: signature_location,
            annotations,
            identifier,
            parameters,
            body,
        })
    }

    fn parse_function_parameters(&mut self) -> Result<Vec<Identifier>> {
        self.consume(Token::LeftParen)?;
        let parameters = if let Token::RightParen = self.peek_token()? {
            Vec::new()
        } else {
            self.parse_identifiers(Token::RightParen)?
        };
        self.consume(Token::RightParen)?;
        Ok(parameters)
    }

    fn parse_identifiers(&mut self, terminator: Token) -> Result<Vec<Identifier>> {
        let mut identifiers = vec![self.parse_identifier()?];
        while self.peek_token()? == Token::Comma {
            self.consume(Token::Comma)?;
            if self.peek_token()? != terminator {
                identifiers.push(self.parse_identifier()?);
            }
        }
        Ok(identifiers)
    }

    fn parse_callback(&mut self, annotations: Vec<Annotation>) -> Result<Decl> {
        self.consume(Token::Callback)?;
        let loc = self.location();
        self.consume(Token::LeftBracket)?;
        let event_type = self.parse_expression(Precedence::Lowest)?;
        self.consume(Token::RightBracket)?;
        self.consume(Token::LeftParen)?;
        let args = if let Token::RightParen = self.peek_token()? {
            Vec::new()
        } else {
            self.parse_comma_separated_expressions(Token::RightParen)?
        };
        self.consume(Token::RightParen)?;
        let body = self.parse_block()?;
        Ok(Decl::Callback {
            location: self.location().merge(&loc),
            annotations,
            event_type,
            args,
            body,
        })
    }

    fn parse_global(&mut self) -> Result<Decl> {
        self.consume(Token::Let)?;
        let start_loc = self.location();
        let ident = self.parse_identifier()?;
        self.consume(Token::Semicolon)?;
        Ok(Decl::Global(self.location().merge(&start_loc), ident))
    }

    fn parse_concrete_stmt(&mut self) -> Result<Stmt> {
        match self.peek_token()? {
            Token::LeftBrace => self.parse_block(),
            Token::Break => self.parse_break(),
            Token::Continue => self.parse_continue(),
            Token::For => self.parse_for(),
            Token::Goto => self.parse_goto(),
            Token::If => self.parse_if(),
            Token::Label => self.parse_label(),
            Token::Let => self.parse_var_decl(),
            Token::Match => self.parse_match(),
            Token::Printf => self.parse_printf(),
            Token::Return => self.parse_return(),
            Token::While => self.parse_while(),
            Token::Yield => self.parse_yield(),
            _ => self.parse_terminated_expr_stmt_or_assignment(),
        }
    }

    fn parse_block(&mut self) -> Result<Stmt> {
        self.consume(Token::LeftBrace)?;
        let loc = self.location();
        let mut contents = Vec::new();
        while self.peek_token()? != Token::RightBrace {
            contents.push(self.parse_concrete_stmt()?);
        }
        self.consume(Token::RightBrace)?;
        Ok(Stmt::Block(self.location().merge(&loc), contents))
    }

    fn parse_break(&mut self) -> Result<Stmt> {
        self.consume(Token::Break)?;
        let loc = self.location();
        self.consume(Token::Semicolon)?;
        Ok(Stmt::Break(loc))
    }

    fn parse_continue(&mut self) -> Result<Stmt> {
        self.consume(Token::Continue)?;
        let loc = self.location();
        self.consume(Token::Semicolon)?;
        Ok(Stmt::Continue(loc))
    }

    fn parse_for(&mut self) -> Result<Stmt> {
        self.consume(Token::For)?;
        let start_loc = self.location();
        self.consume(Token::LeftParen)?;
        let init = Box::new(self.parse_terminated_expr_stmt_or_assignment()?);
        if let Stmt::ExprStmt(location, _) = *init {
            return Err(ParserError::ExpectedAssignment(location));
        }
        let check = self.parse_expression(Precedence::Lowest)?;
        self.consume(Token::Semicolon)?;
        let step = Box::new(self.parse_expr_stmt_or_assignment()?);
        self.consume(Token::RightParen)?;
        let body = Box::new(self.parse_concrete_stmt()?);
        Ok(Stmt::For {
            location: self.location().merge(&start_loc),
            init,
            check,
            step,
            body,
        })
    }

    fn parse_goto(&mut self) -> Result<Stmt> {
        self.consume(Token::Goto)?;
        let start_loc = self.location();
        let identifier = self.parse_identifier()?;
        self.consume(Token::Semicolon)?;
        Ok(Stmt::Goto(
            start_loc.merge(&identifier.location),
            identifier,
        ))
    }

    fn parse_if(&mut self) -> Result<Stmt> {
        self.consume(Token::If)?;
        let start_loc = self.location();
        self.consume(Token::LeftParen)?;
        let condition = self.parse_expression(Precedence::Lowest)?;
        self.consume(Token::RightParen)?;
        let then_part = self.parse_concrete_stmt()?;
        let else_part = if let Token::Else = self.peek_token()? {
            self.consume(Token::Else)?;
            Some(Box::new(self.parse_concrete_stmt()?))
        } else {
            None
        };
        Ok(Stmt::If {
            location: self.location().merge(&start_loc),
            condition,
            then_part: Box::new(then_part),
            else_part,
        })
    }

    fn parse_label(&mut self) -> Result<Stmt> {
        self.consume(Token::Label)?;
        let start_loc = self.location();
        let identifier = self.parse_identifier()?;
        self.consume(Token::Semicolon)?;
        Ok(Stmt::Label(
            start_loc.merge(&identifier.location),
            identifier,
        ))
    }

    fn parse_match(&mut self) -> Result<Stmt> {
        self.consume(Token::Match)?;
        let start_loc = self.location();
        self.consume(Token::LeftParen)?;
        let switch = self.parse_expression(Precedence::Lowest)?;
        self.consume(Token::RightParen)?;
        self.consume(Token::LeftBrace)?;
        let mut cases = Vec::new();
        let mut default = None;
        let mut default_loc = None;
        while self.peek_token()? != Token::RightBrace {
            if let Token::Else = self.peek_token()? {
                self.consume(Token::Else)?;
                let loc = self.location();
                self.consume(Token::Arrow)?;
                if let Some(previous_loc) = default_loc {
                    return Err(ParserError::MultipleDefaultCases(previous_loc, loc));
                }
                default = Some(self.parse_block()?);
                default_loc = Some(loc);
            } else {
                cases.push(self.parse_match_case()?);
            }
        }
        self.consume(Token::RightBrace)?;
        Ok(Stmt::Match {
            location: self.location().merge(&start_loc),
            switch,
            cases,
            default: default.map(Box::new),
        })
    }

    fn parse_match_case(&mut self) -> Result<Case> {
        let conditions = self.parse_comma_separated_expressions(Token::Arrow)?;
        self.consume(Token::Arrow)?;
        let body = self.parse_block()?;
        Ok(Case::new(conditions, body))
    }

    fn parse_printf(&mut self) -> Result<Stmt> {
        self.consume(Token::Printf)?;
        let start_loc = self.location();
        self.consume(Token::LeftParen)?;
        let args = self.parse_comma_separated_expressions(Token::RightParen)?;
        self.consume(Token::RightParen)?;
        self.consume(Token::Semicolon)?;
        Ok(Stmt::Printf(self.location().merge(&start_loc), args))
    }

    fn parse_return(&mut self) -> Result<Stmt> {
        self.consume(Token::Return)?;
        let start_loc = self.location();
        if let Token::Semicolon = self.peek_token()? {
            self.consume(Token::Semicolon)?;
            Ok(Stmt::Return(start_loc, None))
        } else {
            let value = self.parse_expression(Precedence::Lowest)?;
            self.consume(Token::Semicolon)?;
            Ok(Stmt::Return(start_loc.merge(value.location()), Some(value)))
        }
    }

    fn parse_var_decl(&mut self) -> Result<Stmt> {
        self.consume(Token::Let)?;
        let start_loc = self.location();
        let identifier = self.parse_identifier()?;
        self.consume(Token::Semicolon)?;
        Ok(Stmt::VarDecl(self.location().merge(&start_loc), identifier))
    }

    fn parse_while(&mut self) -> Result<Stmt> {
        self.consume(Token::While)?;
        let start_loc = self.location();
        self.consume(Token::LeftParen)?;
        let condition = self.parse_expression(Precedence::Lowest)?;
        self.consume(Token::RightParen)?;
        let body = self.parse_concrete_stmt()?;
        Ok(Stmt::While {
            location: self.location().merge(&start_loc),
            condition,
            body: Box::new(body),
        })
    }

    fn parse_yield(&mut self) -> Result<Stmt> {
        self.consume(Token::Yield)?;
        let loc = self.location();
        self.consume(Token::Semicolon)?;
        Ok(Stmt::Yield(loc))
    }

    // An assignment / expr stmt followed by a semicolon
    // This is its own grammar rule because of for loops ex. for (let i = 0; i < 10; i += 2)
    fn parse_terminated_expr_stmt_or_assignment(&mut self) -> Result<Stmt> {
        let stmt = self.parse_expr_stmt_or_assignment()?;
        self.consume(Token::Semicolon)?;
        Ok(stmt)
    }

    fn parse_expr_stmt_or_assignment(&mut self) -> Result<Stmt> {
        let expr = self.parse_expression(Precedence::Lowest)?;
        match self.peek_token()? {
            Token::Assign => self.parse_assignment(expr, Token::Assign, Operator::Assign),
            Token::AssignAdd => self.parse_assignment(expr, Token::AssignAdd, Operator::AssignAdd),
            Token::AssignSubtract => {
                self.parse_assignment(expr, Token::AssignSubtract, Operator::AssignSubtract)
            }
            Token::AssignMultiply => {
                self.parse_assignment(expr, Token::AssignMultiply, Operator::AssignMultiply)
            }
            Token::AssignDivide => {
                self.parse_assignment(expr, Token::AssignDivide, Operator::AssignDivide)
            }
            Token::AssignModulo => {
                self.parse_assignment(expr, Token::AssignModulo, Operator::AssignModulo)
            }
            Token::AssignBinaryOr => {
                self.parse_assignment(expr, Token::AssignBinaryOr, Operator::AssignBitwiseOr)
            }
            Token::AssignBinaryAnd => {
                self.parse_assignment(expr, Token::AssignBinaryAnd, Operator::AssignBitwiseAnd)
            }
            Token::AssignXor => self.parse_assignment(expr, Token::AssignXor, Operator::AssignXor),
            Token::AssignRightShift => {
                self.parse_assignment(expr, Token::AssignRightShift, Operator::AssignRightShift)
            }
            Token::AssignLeftShift => {
                self.parse_assignment(expr, Token::AssignLeftShift, Operator::AssignLeftShift)
            }
            _ => Ok(Stmt::ExprStmt(expr.location().clone(), expr)),
        }
    }

    fn parse_assignment(&mut self, left: Expr, expected: Token, op: Operator) -> Result<Stmt> {
        match left {
            Expr::Ref(l, r) => {
                self.consume(expected)?;
                let right = if let Token::LeftBracket = self.peek_token()? {
                    self.parse_static_array_init()
                } else if let Token::Array = self.peek_token()? {
                    self.parse_empty_array_init()
                } else {
                    self.parse_expression(Precedence::Lowest)
                }?;
                Ok(Stmt::Assignment {
                    location: l.merge(right.location()),
                    left: r,
                    op,
                    right,
                })
            }
            e => Err(ParserError::ExpectedReference(e.location().clone())),
        }
    }

    fn parse_empty_array_init(&mut self) -> Result<Expr> {
        self.consume(Token::Array)?;
        let loc = self.location();
        self.consume(Token::LeftBracket)?;
        let count = self.parse_expression(Precedence::Lowest)?;
        self.consume(Token::RightBracket)?;
        Ok(Expr::Array(
            self.location().merge(&loc),
            ArrayInit::Empty(Box::new(count)),
        ))
    }

    fn parse_static_array_init(&mut self) -> Result<Expr> {
        self.consume(Token::LeftBracket)?;
        let loc = self.location();
        let values = self.parse_comma_separated_expressions(Token::RightBracket)?;
        self.consume(Token::RightBracket)?;
        Ok(Expr::Array(
            self.location().merge(&loc),
            ArrayInit::Static(values),
        ))
    }

    fn parse_expression(&mut self, precedence: Precedence) -> Result<Expr> {
        let mut expr = self.parse_prefix_expression()?;
        while precedence < self.precedence_of_next()? {
            expr = self.parse_infix_expression(expr)?;
        }
        Ok(expr)
    }

    fn precedence_of_next(&mut self) -> Result<Precedence> {
        Ok(match self.peek_token()? {
            Token::Plus | Token::FloatPlus | Token::Minus | Token::FloatMinus => Precedence::Term,
            Token::Times
            | Token::FloatTimes
            | Token::Divide
            | Token::FloatDivide
            | Token::Modulo => Precedence::Factor,
            Token::Equal | Token::FloatEqual | Token::NotEqual | Token::FloatNotEqual => {
                Precedence::Equality
            }
            Token::LessThan
            | Token::FloatLessThan
            | Token::LessThanOrEqualTo
            | Token::FloatLessThanOrEqualTo
            | Token::GreaterThan
            | Token::FloatGreaterThan
            | Token::GreaterThanOrEqualTo
            | Token::FloatGreaterThanOrEqualTo => Precedence::Comparison,
            Token::RightShift | Token::LeftShift => Precedence::Shift,
            Token::Ampersand | Token::BinaryOr | Token::Xor => Precedence::Bitwise,
            Token::LogicalAnd => Precedence::LogicalAnd,
            Token::LogicalOr => Precedence::LogicalOr,
            Token::Increment | Token::Decrement | Token::LogicalNot | Token::BinaryNot => {
                Precedence::Unary
            }
            _ => Precedence::Lowest,
        })
    }

    fn parse_prefix_expression(&mut self) -> Result<Expr> {
        match self.peek_token()? {
            Token::Times => self.parse_dereference(),
            Token::Ampersand => self.parse_address_of(),
            Token::LogicalNot => self.parse_unary_expr(Token::LogicalNot, Operator::LogicalNot),
            Token::BinaryNot => self.parse_unary_expr(Token::BinaryNot, Operator::BitwiseNot),
            Token::Minus => self.parse_unary_expr(Token::Minus, Operator::Negate),
            Token::FloatMinus => self.parse_unary_expr(Token::FloatMinus, Operator::FloatNegate),
            Token::Increment => self.parse_prefix_increment(Token::Increment, Operator::Increment),
            Token::Decrement => self.parse_prefix_increment(Token::Decrement, Operator::Decrement),
            Token::Identifier => self.parse_identifier_expr(),
            Token::LeftParen => self.parse_grouped(),
            Token::Int => self.parse_int(),
            Token::Float => self.parse_float(),
            Token::Str => self.parse_string(),
            _ => Err(ParserError::ExpectedExpression(self.location())),
        }
    }

    fn parse_dereference(&mut self) -> Result<Expr> {
        self.consume(Token::Times)?;
        let loc = self.location();
        if let Expr::Ref(_, r) = self.parse_expression(Precedence::Unary)? {
            let loc = self.location().merge(&loc);
            match r {
                Ref::Var(id) => Ok(Expr::Ref(loc, Ref::Dereference(id, None))),
                Ref::Index(id, expr) => Ok(Expr::Ref(loc, Ref::Dereference(id, Some(expr)))),
                Ref::Dereference(_, _) => Err(ParserError::DoubleDereference(loc)),
            }
        } else {
            Err(ParserError::ExpectedReference(self.location()))
        }
    }

    fn parse_address_of(&mut self) -> Result<Expr> {
        self.consume(Token::Ampersand)?;
        let loc = self.location();
        if let Expr::Ref(_, r) = self.parse_expression(Precedence::Unary)? {
            Ok(Expr::AddressOf(self.location().merge(&loc), r))
        } else {
            Err(ParserError::ExpectedReference(self.location()))
        }
    }

    fn parse_prefix_increment(&mut self, expected: Token, op: Operator) -> Result<Expr> {
        self.consume(expected)?;
        let start_loc = self.location();
        let operand = self.parse_expression(Precedence::Unary)?;
        match operand {
            Expr::Ref(l, r) => Ok(Expr::Increment(
                start_loc.merge(&l),
                r,
                op,
                Notation::Prefix,
            )),
            e => Err(ParserError::ExpectedReference(e.location().clone())),
        }
    }

    fn parse_identifier_expr(&mut self) -> Result<Expr> {
        let identifier = self.parse_identifier()?;
        if let Token::LeftParen = self.peek_token()? {
            self.consume(Token::LeftParen)?;
            let args = if let Token::RightParen = self.peek_token()? {
                Vec::new()
            } else {
                self.parse_comma_separated_expressions(Token::RightParen)?
            };
            self.consume(Token::RightParen)?;
            Ok(Expr::FunctionCall(
                self.location().merge(&identifier.location),
                identifier,
                args,
            ))
        } else if let Token::LeftBracket = self.peek_token()? {
            self.consume(Token::LeftBracket)?;
            let index = self.parse_expression(Precedence::Lowest)?;
            self.consume(Token::RightBracket)?;
            Ok(Expr::Ref(
                self.location().merge(&identifier.location),
                Ref::Index(identifier, Box::new(index)),
            ))
        } else if let Token::Dot = self.peek_token()? {
            self.consume(Token::Dot)?;
            let variant = self.parse_identifier()?;
            Ok(Expr::EnumAccess(
                self.location().merge(&identifier.location),
                identifier,
                variant,
            ))
        } else {
            Ok(Expr::Ref(identifier.location.clone(), Ref::Var(identifier)))
        }
    }

    fn parse_comma_separated_expressions(&mut self, terminator: Token) -> Result<Vec<Expr>> {
        let mut expressions = vec![self.parse_expression(Precedence::Lowest)?];
        while self.peek_token()? == Token::Comma {
            self.consume(Token::Comma)?;
            if self.peek_token()? != terminator {
                expressions.push(self.parse_expression(Precedence::Lowest)?);
            }
        }
        Ok(expressions)
    }

    fn parse_grouped(&mut self) -> Result<Expr> {
        self.consume(Token::LeftParen)?;
        let start_loc = self.location();
        let expr = self.parse_expression(Precedence::Lowest)?;
        self.consume(Token::RightParen)?;
        Ok(Expr::Grouped(
            self.location().merge(&start_loc),
            Box::new(expr),
        ))
    }

    fn parse_unary_expr(&mut self, expected: Token, op: Operator) -> Result<Expr> {
        let loc = self.location();
        self.consume(expected)?;
        let operand = self.parse_expression(Precedence::Unary)?;
        Ok(Expr::Unary(
            loc.merge(operand.location()),
            Box::new(operand),
            op,
        ))
    }

    fn parse_float(&mut self) -> Result<Expr> {
        self.consume(Token::Float)?;
        let slice = self.lex.slice();
        let value: Option<f32> = slice.parse().ok();
        value
            .map(|v| Expr::Literal(self.location(), Literal::Float(v)))
            .ok_or_else(|| ParserError::InvalidFloat(self.location()))
    }

    fn parse_string(&mut self) -> Result<Expr> {
        self.consume(Token::Str)?;
        Ok(Expr::Literal(
            self.location(),
            Literal::Str(self.lex.slice()[1..self.lex.slice().len() - 1].to_owned()),
        ))
    }

    fn parse_int(&mut self) -> Result<Expr> {
        self.consume(Token::Int)?;
        let slice = self.lex.slice();
        let value: Option<i32> = if slice.starts_with("0x") {
            i32::from_str_radix(slice.strip_prefix("0x").unwrap(), 16).ok()
        } else if slice.starts_with("0o") {
            i32::from_str_radix(slice.strip_prefix("0o").unwrap(), 8).ok()
        } else if slice.starts_with("0b") {
            i32::from_str_radix(slice.strip_prefix("0b").unwrap(), 2).ok()
        } else {
            slice.parse().ok()
        };
        value
            .map(|v| Expr::Literal(self.location(), Literal::Int(v)))
            .ok_or_else(|| ParserError::InvalidInt(self.location()))
    }

    fn parse_infix_expression(&mut self, left: Expr) -> Result<Expr> {
        match self.peek_token()? {
            Token::Plus => self.parse_binary_expr(left, Token::Plus, Operator::Add),
            Token::FloatPlus => self.parse_binary_expr(left, Token::FloatPlus, Operator::FloatAdd),
            Token::Minus => self.parse_binary_expr(left, Token::Minus, Operator::Subtract),
            Token::FloatMinus => {
                self.parse_binary_expr(left, Token::FloatMinus, Operator::FloatSubtract)
            }
            Token::Times => self.parse_binary_expr(left, Token::Times, Operator::Multiply),
            Token::FloatTimes => {
                self.parse_binary_expr(left, Token::FloatTimes, Operator::FloatMultiply)
            }
            Token::Divide => self.parse_binary_expr(left, Token::Divide, Operator::Divide),
            Token::FloatDivide => {
                self.parse_binary_expr(left, Token::FloatDivide, Operator::FloatDivide)
            }
            Token::Modulo => self.parse_binary_expr(left, Token::Modulo, Operator::Modulo),
            Token::Equal => self.parse_binary_expr(left, Token::Equal, Operator::Equal),
            Token::FloatEqual => {
                self.parse_binary_expr(left, Token::FloatEqual, Operator::FloatEqual)
            }
            Token::NotEqual => self.parse_binary_expr(left, Token::NotEqual, Operator::NotEqual),
            Token::FloatNotEqual => {
                self.parse_binary_expr(left, Token::FloatNotEqual, Operator::FloatNotEqual)
            }
            Token::LessThan => self.parse_binary_expr(left, Token::LessThan, Operator::LessThan),
            Token::FloatLessThan => {
                self.parse_binary_expr(left, Token::FloatLessThan, Operator::FloatLessThan)
            }
            Token::LessThanOrEqualTo => {
                self.parse_binary_expr(left, Token::LessThanOrEqualTo, Operator::LessThanEqualTo)
            }
            Token::FloatLessThanOrEqualTo => self.parse_binary_expr(
                left,
                Token::FloatLessThanOrEqualTo,
                Operator::FloatLessThanEqualTo,
            ),
            Token::GreaterThan => {
                self.parse_binary_expr(left, Token::GreaterThan, Operator::GreaterThan)
            }
            Token::FloatGreaterThan => {
                self.parse_binary_expr(left, Token::FloatGreaterThan, Operator::FloatGreaterThan)
            }
            Token::GreaterThanOrEqualTo => self.parse_binary_expr(
                left,
                Token::GreaterThanOrEqualTo,
                Operator::GreaterThanEqualTo,
            ),
            Token::FloatGreaterThanOrEqualTo => self.parse_binary_expr(
                left,
                Token::FloatGreaterThanOrEqualTo,
                Operator::FloatGreaterThanEqualTo,
            ),
            Token::RightShift => {
                self.parse_binary_expr(left, Token::RightShift, Operator::RightShift)
            }
            Token::LeftShift => self.parse_binary_expr(left, Token::LeftShift, Operator::LeftShift),
            Token::Ampersand => {
                self.parse_binary_expr(left, Token::Ampersand, Operator::BitwiseAnd)
            }
            Token::BinaryOr => self.parse_binary_expr(left, Token::BinaryOr, Operator::BitwiseOr),
            Token::Xor => self.parse_binary_expr(left, Token::Xor, Operator::Xor),
            Token::LogicalAnd => {
                self.parse_binary_expr(left, Token::LogicalAnd, Operator::LogicalAnd)
            }
            Token::LogicalOr => self.parse_binary_expr(left, Token::LogicalOr, Operator::LogicalOr),
            Token::BinaryNot => {
                self.parse_binary_expr(left, Token::BinaryNot, Operator::BitwiseNot)
            }
            Token::LogicalNot => {
                self.parse_binary_expr(left, Token::LogicalNot, Operator::LogicalNot)
            }
            Token::Increment => {
                self.parse_postfix_increment(left, Token::Increment, Operator::Increment)
            }
            Token::Decrement => {
                self.parse_postfix_increment(left, Token::Decrement, Operator::Decrement)
            }
            _ => Err(ParserError::ExpectedExpression(self.location())),
        }
    }

    fn parse_postfix_increment(
        &mut self,
        left: Expr,
        expected: Token,
        op: Operator,
    ) -> Result<Expr> {
        match left {
            Expr::Ref(l, r) => {
                self.consume(expected)?;
                Ok(Expr::Increment(
                    self.location().merge(&l),
                    r,
                    op,
                    Notation::Postfix,
                ))
            }
            e => Err(ParserError::ExpectedReference(e.location().clone())),
        }
    }

    fn parse_binary_expr(&mut self, left: Expr, expected: Token, op: Operator) -> Result<Expr> {
        self.consume(expected)?;
        let right = self.parse_expression(op.into())?;
        Ok(Expr::Binary(
            left.location().merge(right.location()),
            Box::new(left),
            op,
            Box::new(right),
        ))
    }

    /// Consume the next token and convert it to an identifier.
    /// Errors if the next token is not an identifier.
    fn parse_identifier(&mut self) -> Result<Identifier> {
        self.consume(Token::Identifier)?;
        Ok(Identifier::new(
            self.location(),
            self.lex.slice().to_owned(),
        ))
    }

    /// Consume a token and error if its not the expected token.
    fn consume(&mut self, expected: Token) -> Result<()> {
        let actual = self.next_token()?;
        if expected == actual {
            Ok(())
        } else {
            Err(ParserError::UnexpectedToken(
                self.location(),
                expected.to_string(),
                actual.to_string(),
            ))
        }
    }

    /// Move the lexer forward one token.
    fn next_token(&mut self) -> Result<Token> {
        let token = self.lex.next().ok_or(ParserError::UnexpectedEof)?;
        if let Token::Error = token {
            Err(ParserError::InvalidToken(self.location()))
        } else {
            Ok(token)
        }
    }

    /// Look at the next token without moving forward.
    fn peek_token(&mut self) -> Result<Token> {
        let token = self.lex.peek().ok_or(ParserError::UnexpectedEof)?;
        if let Token::Error = token {
            Err(ParserError::InvalidToken(self.location()))
        } else {
            Ok(token)
        }
    }

    /// Get the current location in the source code.
    fn location(&self) -> Location {
        Location::Source(self.file_id, self.lex.span())
    }
}

pub fn parse<'a, 'source>(
    file_id: FileId,
    source: &'source str,
    log: &'a mut CompilerLog<'source>,
) -> Script {
    let mut parser = Parser::new(file_id, source, log);
    parser.parse_script()
}
