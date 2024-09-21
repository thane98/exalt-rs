use exalt_compiler::SymbolTable;

/// Completion server for a single script file.
/// Provides suggestions using cached data from a successful run of the parser.
#[derive(Default)]
pub struct CompletionServer {
    symbols: Vec<String>,
}

impl CompletionServer {
    pub fn from_symbol_table(symbol_table: &SymbolTable) -> Self {
        Self {
            symbols: symbol_table
                .constants()
                .iter()
                .map(|c| c.borrow().name.clone())
                .chain(symbol_table.enums().iter().map(|s| s.borrow().name.clone()))
                .chain(
                    symbol_table
                        .functions()
                        .iter()
                        .map(|c| c.borrow().name.clone()),
                )
                .collect(),
        }
    }

    pub fn suggest_completions(&self, prefix: &str) -> Vec<&str> {
        self.symbols
            .iter()
            .filter(|s| s.starts_with(prefix))
            .map(|s| s.as_str())
            .collect()
    }
}
