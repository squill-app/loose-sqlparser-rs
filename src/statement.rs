use crate::tokens::Tokens;

pub struct SqlStatement<'s> {
    // The SQL statement.
    pub(crate) sql: &'s str,

    // The line and column where the statement starts.
    pub(crate) start_line: usize,
    pub(crate) start_column: usize,

    // A list of tokens found in the statement at the top level.
    // Tokens found on CTEs or sub queries are not included in this list.
    pub(crate) tokens: Tokens<'s>,
}

impl SqlStatement<'_> {
    /// The SQL statement.
    pub fn sql(&self) -> &str {
        self.sql
    }

    /// The line where the statement starts.
    pub fn start_line(&self) -> usize {
        self.start_line
    }

    /// The column where the statement starts.
    pub fn start_column(&self) -> usize {
        self.start_column
    }

    pub fn tokens(&self) -> &Tokens<'_> {
        &self.tokens
    }

    /// The list of keywords found in the statement at the top level.
    /// Keywords found on CTEs or sub queries are not included in this list.
    pub fn keywords(&self) -> Vec<&str> {
        self.tokens
            .as_str_array()
            .iter()
            .filter(|&&token| token.chars().all(|c| c.is_ascii_alphabetic()))
            .cloned() // Clone the &str references to return a Vec<&'s str>
            .collect()
    }

    /// Returns whether the statement is a query or a command.
    ///
    /// The following SQL statements are considered queries:
    /// - SELECT ... (excluding SELECT INTO)
    /// - SHOW ...
    /// - DESCRIBE ...
    /// - EXPLAIN ...
    /// - WITH ... SELECT ...
    /// - VALUES ...
    /// - LIST ...
    /// - SHOW ...
    /// - PRAGMA ...
    /// - INSERT|UPDATE|DELETE ... RETURNING ...
    pub fn is_query(&self) -> bool {
        let keywords = self.keywords();
        // 1. The statement starts with a keyword that is unambiguously a query.
        (matches!(keywords[0].to_uppercase().as_str(),
            "SHOW" | "DESCRIBE" | "EXPLAIN" | "VALUES" | "LIST" | "PRAGMA"))
        // 2. The statement starts with a WITH clause followed by a SELECT.
            || (keywords[0].to_uppercase() == "WITH"
                && keywords.iter().any(|&k| matches!(k.to_uppercase().as_str(), "SELECT")))
        // 3. The statement is an INSERT, UPDATE, or DELETE with a RETURNING clause.
            || (matches!(keywords[0].to_uppercase().as_str(), "INSERT" | "UPDATE" | "DELETE")
                && keywords.iter().any(|&k| k.to_uppercase().as_str() == "RETURNING"))
        // 4. The statement is a SELECT (except SELECT ... INTO).
            || (keywords[0].to_uppercase() == "SELECT"
                && keywords.iter().any(|&k| k.to_uppercase().as_str() == "INTO"))
    }
}
