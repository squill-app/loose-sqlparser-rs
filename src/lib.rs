pub mod statement;
mod tokenizer;
pub mod tokens;

use statement::SqlStatement;
use tokenizer::Tokenizer;

/// Represents a position in the input string.
#[derive(Debug)]
pub struct Position {
    /// Line number (1-based).
    pub line: usize,

    /// Column number (1-based).
    pub column: usize,

    /// Offset in the input string (0-based)
    /// The offset is the number of characters (not bytes) from the start.
    pub offset: usize,
}

/// Scans a SQL string and returns an iterator over the statements.
///
/// This is a non-validating SQL parser, it will not check the syntax validity of the SQL statements.
///
/// The iterator will return a {{SqlStatement}} for each statement found in the input string.
/// Statements are separated by a semicolon (`;`).
pub fn loose_sqlparse(sql: &str) -> impl Iterator<Item = SqlStatement<'_>> {
    Tokenizer::new(sql, ";")
}

/// Scans a SQL string and returns an iterator over the statements.
///
/// This is a non-validating SQL parser, it will not check the syntax validity of the SQL statements.
///
/// The iterator will return a {{SqlStatement}} for each statement found in the input string.
/// Statements are separated by the given delimiter.
pub fn loose_sqlparse_with_delimiter<'s>(sql: &'s str, delimiter: &'s str) -> impl Iterator<Item = SqlStatement<'s>> {
    Tokenizer::new(sql, delimiter)
}
