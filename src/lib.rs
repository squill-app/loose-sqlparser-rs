#![doc = include_str!("../README.md")]

mod options;
mod statement;
mod tokenizer;
mod tokens;

// Re-export the public API
pub use options::Options;
pub use statement::Statement;
pub use tokens::{Token, TokenValue, Tokens};

use tokenizer::Tokenizer;

/// A position in the input string given to the parser.
///
/// # Examples
///
/// ```rust
/// use loose_sqlparser::loose_sqlparse;
/// let stmt = loose_sqlparse("SELECT 1;\nSELECT 2;").nth(1).unwrap();
/// assert_eq!(stmt.sql(), "SELECT 2;");
/// assert_eq!(stmt.start().line, 2);
/// assert_eq!(stmt.start().column, 1);
/// assert_eq!(stmt.start().offset, 10);
/// ```
#[derive(Debug, Clone)]
pub struct Position {
    /// Line number (1-based).
    pub line: usize,

    /// Column number (1-based).
    pub column: usize,

    /// Offset in the input string (0-based)
    /// The offset is the number of characters (not bytes) from the start of the first character of the token.
    pub offset: usize,
}

/// Scans a SQL string and returns an iterator over the statements.
///
/// This is a non-validating SQL parser, it will not check the syntax validity of the SQL statements.
///
/// The iterator will return a {{SqlStatement}} for each statement found in the input string.
/// Statements are separated by a semicolon (`;`).
pub fn loose_sqlparse(sql: &str) -> impl Iterator<Item = Statement<'_>> {
    Tokenizer::new(sql, Options::default())
}

/// Scans a SQL string and returns an iterator over the statements.
///
/// This is a non-validating SQL parser, it will not check the syntax validity of the SQL statements.
///
/// The iterator will return a {{Statement}} for each statement found in the input string.
/// Statements are separated by the given delimiter.
pub fn loose_sqlparse_with_options(sql: &str, options: Options) -> impl Iterator<Item = Statement<'_>> {
    Tokenizer::new(sql, options)
}

/// Alias of {{loose_sqlparse}}.
pub fn parse(sql: &str) -> impl Iterator<Item = Statement<'_>> {
    Tokenizer::new(sql, Options::default())
}

/// Alias of {{loose_sqlparse_with_options}}.
pub fn parse_with_options(sql: &str, options: Options) -> impl Iterator<Item = Statement<'_>> {
    Tokenizer::new(sql, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api() {
        let statements: Vec<_> = loose_sqlparse("SELECT /* one */ 1;SELECT 2").collect();
        assert_eq!(statements[0].tokens().as_str_array(), ["SELECT", "/* one */", "1", ";"]);
        assert_eq!(statements[1].tokens().as_str_array(), ["SELECT", "2"]);

        let statements: Vec<_> = parse("SELECT /* one */ 1;SELECT 2").collect();
        assert_eq!(statements[0].tokens().as_str_array(), ["SELECT", "/* one */", "1", ";"]);
        assert_eq!(statements[1].tokens().as_str_array(), ["SELECT", "2"]);

        let options = Options { statement_delimiter: "\\".to_string() };
        let statements: Vec<_> = loose_sqlparse_with_options("SELECT /* one */ 1\\SELECT 2", options).collect();
        assert_eq!(statements[0].tokens().as_str_array(), ["SELECT", "/* one */", "1", "\\"]);
        assert_eq!(statements[1].tokens().as_str_array(), ["SELECT", "2"]);

        let options = Options { statement_delimiter: "\\".to_string() };
        let statements: Vec<_> = parse_with_options("SELECT /* one */ 1\\SELECT 2", options).collect();
        assert_eq!(statements[0].tokens().as_str_array(), ["SELECT", "/* one */", "1", "\\"]);
        assert_eq!(statements[1].tokens().as_str_array(), ["SELECT", "2"]);
    }

    #[test]
    fn test_loose_sqlparse_with_options() {}
}
