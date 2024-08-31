use crate::Position;
use std::convert::AsRef;
use std::ops::{Deref, DerefMut};

#[cfg(feature = "serialize")]
use serde::{ser::SerializeStruct, Serialize, Serializer};

// A token extracted from the input string.
#[derive(Debug)]
pub enum TokenValue<'s> {
    /// Any token that does not match any of the other variants.
    Any(&'s str),

    /// A comment.
    ///
    /// - Single-line comments start with `--` or '#' and continue to the end of the line.
    /// - Multi-line comments start with `/*` and end with `*/`.
    Comment(&'s str),

    /// A quoted identifier or a non numeric constant.
    ///
    /// - *Quoted identifiers* are enclosed in double quotes (`"`). They are identifiers (like a table name, column name,
    ///   or other object) that might otherwise conflict with SQL syntax rules or keywords.
    ///
    ///   ```sql
    ///   -- "ORDER BY" is a quoted identifier
    ///   SELECT 1 as "ORDER BY" FROM DUAl;
    ///   ```
    ///
    ///   Notes:
    ///     - MySQL and MariaDB are also allowing backticks (`` ` ``) and single quotes (`'`) for quoting identifiers.
    ///     - SQL Server is also allowing square brackets (`[]`) for quoting identifiers.
    ///
    /// - *String constants* are enclosed in single quotes (`'`). They are used to represent string literals.
    ///
    ///   ```sql
    ///   -- 'Hello World' is a string constant.
    ///   SELECT 'Hello World' FROM DUAl;
    ///   ```
    ///
    ///   Notes:
    ///     - MySQL and MariaDB are also allowing single quotes (`'`) for string literals.
    ///     - PostgreSQL is also allowing dollar-quoted strings (`$tag$...$tag$`) for string literals.
    QuotedIdentifierOrConstant(&'s str),

    /// A Numeric Constant
    ///
    /// - A numeric constant is a sequence of digits with an optional decimal point and an optional exponent.
    ///   ```text
    ///   digits
    ///   digits.[digits][e[+-]digits]
    ///   [digits].digits[e[+-]digits]
    ///   digitse[+-]digits
    ///   ```
    /// - Hexadecimal constants (`0xhexdigits`).
    /// - Binary constants (`0b01`).
    /// - Octal constants (`0o01234567`).
    ///
    /// For visual grouping, underscores can be inserted between digits (e.g. `1_000_000`).
    ///
    /// Examples:
    ///
    /// ```sql
    /// 42
    /// 3.5
    /// 4.
    /// .001
    /// 5e2
    /// 1.925e-3
    /// 0b100101
    /// 0B10011001
    /// 0o273
    /// 0O755
    /// 0x42f
    /// 0XFFFF
    /// 1_500_000_000
    /// 0b10001000_00000000
    /// 0o_1_755
    /// 0xFFFF_FFFF
    /// 1.618_034
    /// ```
    NumericConstant(&'s str),

    /// An identifier or a keyword.
    ///
    /// SQL identifiers and key words must begin with a letter (a-z, but also letters with diacritical marks and
    /// non-Latin letters) or an underscore (_). Subsequent characters in an identifier or key word can be letters,
    /// underscores, digits (0-9), or dollar signs ($).
    IdentifierOrKeyword(&'s str),

    /// An operator
    ///
    /// - Arithmetic operators: `+`, `-`, `*`, `/`, `=`, `!=`, `>`, `<`, `>=`, `<=`, `<>`, `||`, `!`, `%`
    /// - Bitwise operators: `~`, `&`, `|`, `<<`, `>>`, `^`
    /// - PostgreSQL typecast operator: `::`
    /// - Regular expression operators: `~`, `~*`, `!~`, `!~*`
    Operator(&'s str),

    /// Mark the end of an SQL statement.
    ///
    /// The default statement delimiter is a semicolon (`;`), but it can be changed in [`crate::Options`].
    StatementDelimiter(&'s str),

    /// Parameter Marker
    ///
    /// Parameter markers indicates where data values are to be bound to the query later when executed.
    ///
    /// ```sql
    /// SELECT * FROM users WHERE id = ? AND name = ?;
    /// ```
    ///
    /// - Question Mark (`?`) Syntax: Widely used in databases like SQLite, MySQL, PostgreSQL.
    /// - Dollar Sign (`$n`) Syntax: PostgreSQL.
    /// - Named Parameters with (`:`), (`$`) or (`@`) Syntax (ex: `:user_id`, `$user_id`, `@user_id`).
    ParameterMarker(&'s str),

    /// A fragment of tokens, typically used for the content of parenthesis.
    Fragment(Tokens<'s>),
}

impl<'s> AsRef<str> for TokenValue<'s> {
    fn as_ref(&self) -> &str {
        match self {
            TokenValue::Any(value) => value,
            TokenValue::Comment(value) => value,
            TokenValue::QuotedIdentifierOrConstant(value) => value,
            TokenValue::Operator(value) => value,
            TokenValue::StatementDelimiter(value) => value,
            TokenValue::NumericConstant(value) => value,
            TokenValue::IdentifierOrKeyword(value) => value,
            TokenValue::ParameterMarker(value) => value,
            TokenValue::Fragment(_) => {
                panic!("TokenValue::Fragment does not contain a single &str")
            }
        }
    }
}

#[derive(Debug)]
pub struct Token<'s> {
    /// The value of the token.
    pub value: TokenValue<'s>,

    /// The position of the token's first character.
    pub start: Position,

    /// The position of the token's last character.
    pub end: Position,
}

impl<'s> Token<'s> {
    pub fn new(value: TokenValue<'s>, start: Position, end: Position) -> Self {
        Self { value, start, end }
    }

    pub fn is_any(&self) -> bool {
        matches!(self.value, TokenValue::Any(_))
    }

    pub fn is_numeric_constant(&self) -> bool {
        matches!(self.value, TokenValue::NumericConstant(_))
    }

    pub fn is_comment(&self) -> bool {
        matches!(self.value, TokenValue::Comment(_))
    }

    pub fn is_quoted_identifier_or_constant(&self) -> bool {
        matches!(self.value, TokenValue::QuotedIdentifierOrConstant(_))
    }

    pub fn is_fragment(&self) -> bool {
        matches!(self.value, TokenValue::Fragment(_))
    }

    pub fn is_statement_delimiter(&self) -> bool {
        matches!(self.value, TokenValue::StatementDelimiter(_))
    }

    pub fn is_operator(&self) -> bool {
        matches!(self.value, TokenValue::Operator(_))
    }

    pub fn is_parenthesis(&self) -> bool {
        match &self.value {
            TokenValue::Any(value) => *value == "(" || *value == ")",
            _ => false,
        }
    }

    pub fn is_comma(&self) -> bool {
        match &self.value {
            TokenValue::Any(value) => *value == ",",
            _ => false,
        }
    }

    pub fn is_identifier_or_keyword(&self) -> bool {
        matches!(self.value, TokenValue::IdentifierOrKeyword(_))
    }

    pub fn is_parameter_marker(&self) -> bool {
        matches!(self.value, TokenValue::ParameterMarker(_))
    }

    pub fn children(&self) -> Option<&Tokens<'s>> {
        match &self.value {
            TokenValue::Fragment(tokens) => Some(tokens),
            _ => None,
        }
    }

    /// Return the token value as a string array.
    pub fn as_str_array(&self) -> Vec<&str> {
        match &self.value {
            TokenValue::Any(value) => vec![value],
            TokenValue::Comment(value) => vec![value],
            TokenValue::QuotedIdentifierOrConstant(value) => vec![value],
            TokenValue::StatementDelimiter(value) => vec![value],
            TokenValue::Operator(value) => vec![value],
            TokenValue::NumericConstant(value) => vec![value],
            TokenValue::IdentifierOrKeyword(value) => vec![value],
            TokenValue::ParameterMarker(value) => vec![value],
            TokenValue::Fragment(tokens) => tokens.iter().flat_map(|t| t.as_str_array()).collect(),
        }
    }
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            TokenValue::Fragment(tokens) => {
                for token in tokens.iter() {
                    write!(f, "{}", token)?;
                }
                Ok(())
            }
            _ => write!(f, "{}", self.value.as_ref()),
        }
    }
}

#[cfg(feature = "serialize")]
macro_rules! ser_token_value {
    ($state:expr, $variant:ident, $value:expr) => {{
        $state.serialize_field("type", stringify!($variant))?;
        $state.serialize_field("value", $value)?;
    }};
}

#[cfg(feature = "serialize")]
impl<'s> Serialize for Token<'s> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Token", 4)?;
        match &self.value {
            TokenValue::Any(value) => ser_token_value!(state, Any, value),
            TokenValue::Comment(value) => ser_token_value!(state, Comment, value),
            TokenValue::QuotedIdentifierOrConstant(value) => ser_token_value!(state, QuotedIdentifierOrConstant, value),
            TokenValue::Operator(value) => ser_token_value!(state, Operator, value),
            TokenValue::StatementDelimiter(value) => ser_token_value!(state, StatementDelimiter, value),
            TokenValue::NumericConstant(value) => ser_token_value!(state, NumericConstant, value),
            TokenValue::IdentifierOrKeyword(value) => ser_token_value!(state, IdentifierOrKeyword, value),
            TokenValue::ParameterMarker(value) => ser_token_value!(state, ParameterMarker, value),
            TokenValue::Fragment(tokens) => {
                state.serialize_field("type", "Fragment")?;
                state.serialize_field("value", &tokens)?;
            }
        }
        state.serialize_field("start", &self.start)?;
        state.serialize_field("end", &self.end)?;
        state.end()
    }
}

/// A collection of tokens.
#[derive(Debug, Default)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Tokens<'s>(Vec<Token<'s>>);

impl<'s> Tokens<'s> {
    /// Create a new empty Tokens collection.
    pub fn new() -> Self {
        Tokens(Vec::new())
    }

    /// Returns the tokens as a string array.
    ///
    /// # Examples
    /// ```rust
    /// use loose_sqlparser::loose_sqlparse;
    /// let stmt = loose_sqlparse("SELECT 1, 2").next().unwrap();
    /// let tokens = stmt.tokens();
    /// assert_eq!(tokens.as_str_array(), &["SELECT", "1", ",", "2"]);
    /// ```
    pub fn as_str_array(&self) -> Vec<&str> {
        self.iter().flat_map(|t| t.as_str_array()).collect()
    }
}

// Implement Deref to delegate method calls to the inner Vec<Token<'s>>
impl<'s> Deref for Tokens<'s> {
    type Target = Vec<Token<'s>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Implement DerefMut to allow mutable access to the inner Vec<Token<'s>>
impl<'s> DerefMut for Tokens<'s> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_helper_functions() {
        assert!(Token::new(TokenValue::Any("."), Position::new(1, 1, 0), Position::new(1, 1, 1)).is_any());
        assert!(Token::new(TokenValue::NumericConstant("42"), Position::new(1, 1, 0), Position::new(1, 2, 1))
            .is_numeric_constant());
        assert!(Token::new(TokenValue::Comment("--"), Position::new(1, 1, 0), Position::new(1, 3, 2)).is_comment());
        assert!(Token::new(
            TokenValue::QuotedIdentifierOrConstant("'Hello'"),
            Position::new(1, 1, 0),
            Position::new(1, 8, 7)
        )
        .is_quoted_identifier_or_constant());
        assert!(Token::new(TokenValue::Fragment(Tokens::new()), Position::new(1, 1, 0), Position::new(1, 1, 0))
            .is_fragment());
        assert!(Token::new(TokenValue::StatementDelimiter(";"), Position::new(1, 1, 0), Position::new(1, 1, 0))
            .is_statement_delimiter());
        assert!(Token::new(TokenValue::Operator("+"), Position::new(1, 1, 0), Position::new(1, 1, 0)).is_operator());
        assert!(Token::new(TokenValue::Any("("), Position::new(1, 1, 0), Position::new(1, 1, 0)).is_parenthesis());
        assert!(Token::new(TokenValue::Any(")"), Position::new(1, 1, 0), Position::new(1, 1, 0)).is_parenthesis());
        assert!(!Token::new(TokenValue::Any("}"), Position::new(1, 1, 0), Position::new(1, 1, 0)).is_parenthesis());
        assert!(!Token::new(TokenValue::Operator("+"), Position::new(1, 1, 0), Position::new(1, 1, 0)).is_parenthesis());
        assert!(Token::new(TokenValue::Any(","), Position::new(1, 1, 0), Position::new(1, 1, 0)).is_comma());
        assert!(!Token::new(TokenValue::Any("."), Position::new(1, 1, 0), Position::new(1, 1, 0)).is_comma());
        assert!(!Token::new(TokenValue::Operator("+"), Position::new(1, 1, 0), Position::new(1, 1, 0)).is_comma());
        assert!(Token::new(TokenValue::IdentifierOrKeyword("SELECT"), Position::new(1, 1, 0), Position::new(1, 6, 5))
            .is_identifier_or_keyword());
        assert!(Token::new(TokenValue::ParameterMarker("?"), Position::new(1, 1, 0), Position::new(1, 1, 0))
            .is_parameter_marker());
    }

    #[test]
    fn test_children() {
        assert!(Token::new(TokenValue::Fragment(Tokens::new()), Position::new(1, 1, 0), Position::new(1, 1, 0))
            .children()
            .is_some());
        assert!(Token::new(TokenValue::Any("SELECT"), Position::new(1, 1, 0), Position::new(1, 6, 5))
            .children()
            .is_none());
    }
}
