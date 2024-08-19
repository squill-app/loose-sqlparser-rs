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
/// A position could be the start or the end of a token. In both case, line and column match the position of the
/// character in the input string (first or last character of the token). Offset on the other hand differs between
/// start and end. For the start of a token, the offset is the number of bytes from the start of the input string to the
/// first character of the token. For the end of a token, the offset is the number of bytes from the start of the input
/// plus the number of bytes of the token. This basically means that the offset of the end of a token is the offset of
/// next character after the token, allowing to easily get the token's content with `&input[start.offset..end.offset]`.
///
/// # Examples
///
/// ```rust
/// use loose_sqlparser::loose_sqlparse;
/// let input = "SELECT 1;\nSELECT 2;";
/// let stmt = loose_sqlparse(input).nth(1).unwrap();
/// assert_eq!(stmt.sql(), "SELECT 2;");
/// assert_eq!(stmt.start().line, 2);
/// assert_eq!(stmt.start().column, 1);
/// assert_eq!(stmt.start().offset, 10);
/// assert_eq!(&input[stmt.tokens()[1].start.offset..stmt.tokens()[1].end.offset], "2");
/// ```
#[derive(Debug, Clone)]
pub struct Position {
    /// Line number (1-based).
    pub line: usize,

    /// Column number (1-based).
    pub column: usize,

    /// Offset in the input string (0-based)
    /// The offset is the number of bytes (not characters) from the start of the input string.
    pub offset: usize,
}

impl Position {
    /// Create a new position.
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }
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
