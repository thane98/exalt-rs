use std::collections::HashMap;

use exalt_ast::{
    ConstSymbol, EnumSymbol, FunctionSymbol, LabelSymbol, Location, Shared, VarSymbol,
};
use crate::reporting::SemanticError;

type Result<T> = std::result::Result<T, SemanticError>;

/// We group const/var because they can be used at the same points in expressions
#[derive(Debug, Clone)]
pub(crate) enum Variable {
    Const(Shared<ConstSymbol>),
    Var(Shared<VarSymbol>),
}

impl Variable {
    pub fn definition_location(&self) -> Location {
        match self {
            Variable::Const(c) => c.borrow().location.clone(),
            Variable::Var(v) => v.borrow().location.clone(),
        }
    }
}

/// Container for symbols within a single scope
struct Scope {
    variables: HashMap<String, Variable>,
    labels: HashMap<String, Shared<LabelSymbol>>,
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            variables: HashMap::new(),
            labels: HashMap::new(),
        }
    }

    pub fn define_variable(&mut self, name: String, var: Variable) -> Result<()> {
        match self.variables.get(&name) {
            Some(original) => Err(SemanticError::SymbolRedefinition(
                original.definition_location(),
                var.definition_location(),
                name,
            )),
            None => {
                self.variables.insert(name, var);
                Ok(())
            }
        }
    }

    pub fn define_label(&mut self, name: String, label: Shared<LabelSymbol>) -> Result<()> {
        match self.labels.get(&name) {
            Some(original) => Err(SemanticError::SymbolRedefinition(
                original.borrow().location.clone(),
                label.borrow().location.clone(),
                name,
            )),
            None => {
                self.labels.insert(name, label);
                Ok(())
            }
        }
    }

    pub fn lookup_variable(&self, name: &str) -> Option<Variable> {
        self.variables.get(name).map(|v| v.to_owned())
    }

    pub fn lookup_label(&self, name: &str) -> Option<Shared<LabelSymbol>> {
        self.labels.get(name).map(|l| l.to_owned())
    }
}

/// Used to always define labels in the scope of the current function
/// Value is 1 because global scope is 0 and the function is one scope further
const FUNCTION_SCOPE: usize = 1;

/// Data structure for all symbols in the current context
pub(crate) struct SymbolTable {
    scopes: Vec<Scope>,
    enums: HashMap<String, Shared<EnumSymbol>>,
    functions: HashMap<String, Shared<FunctionSymbol>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        // Set up built in functions
        let mut functions = HashMap::new();
        functions.insert("negate".to_owned(), FunctionSymbol::shared("negate".to_owned(), Location::Generated, 1));
        functions.insert("fix".to_owned(), FunctionSymbol::shared("fix".to_owned(), Location::Generated, 1));
        functions.insert("float".to_owned(), FunctionSymbol::shared("float".to_owned(), Location::Generated, 1));
        functions.insert("streq".to_owned(), FunctionSymbol::shared("streq".to_owned(), Location::Generated, 2));
        functions.insert("strne".to_owned(), FunctionSymbol::shared("strne".to_owned(), Location::Generated, 2));

        SymbolTable {
            scopes: vec![Scope::new()],
            enums: HashMap::new(),
            functions,
        }
    }

    pub fn open_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn close_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define_enum(&mut self, name: String, symbol: Shared<EnumSymbol>) -> Result<()> {
        match self.enums.get(&name) {
            Some(original) => Err(SemanticError::SymbolRedefinition(
                original.borrow().location.clone(),
                symbol.borrow().location.clone(),
                name,
            )),
            None => {
                self.enums.insert(name, symbol);
                Ok(())
            }
        }
    }

    pub fn define_function(&mut self, name: String, symbol: Shared<FunctionSymbol>) -> Result<()> {
        match self.functions.get(&name) {
            Some(original) => Err(SemanticError::SymbolRedefinition(
                original.borrow().location.clone(),
                symbol.borrow().location.clone(),
                name,
            )),
            None => {
                self.functions.insert(name, symbol);
                Ok(())
            }
        }
    }

    pub fn define_variable(&mut self, name: String, var: Variable) -> Result<()> {
        self.scopes.last_mut().unwrap().define_variable(name, var)
    }

    pub fn define_label(&mut self, name: String, symbol: Shared<LabelSymbol>) -> Result<()> {
        self.scopes[FUNCTION_SCOPE].define_label(name, symbol)
    }

    pub fn lookup_enum(&self, name: &str) -> Option<Shared<EnumSymbol>> {
        self.enums.get(name).cloned()
    }

    pub fn lookup_function(&self, name: &str) -> Option<Shared<FunctionSymbol>> {
        self.functions.get(name).map(|f| f.to_owned())
    }

    pub fn lookup_variable(&self, name: &str) -> Option<Variable> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.lookup_variable(name) {
                return Some(v);
            }
        }
        None
    }

    pub fn lookup_label(&self, name: &str) -> Option<Shared<LabelSymbol>> {
        self.scopes[FUNCTION_SCOPE].lookup_label(name)
    }
}
