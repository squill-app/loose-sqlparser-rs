use crate::Position;
use std::convert::AsRef;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub enum TokenValue<'s> {
    Any(&'s str),
    Comment(&'s str),
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

    /// A statement delimiter such as `;`.
    StatementDelimiter(&'s str),

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
            TokenValue::Any(value) => value == &"(" || value == &")",
            _ => false,
        }
    }

    pub fn is_comma(&self) -> bool {
        match &self.value {
            TokenValue::Any(value) => value == &",",
            _ => false,
        }
    }

    pub fn is_identifier_or_keyword(&self) -> bool {
        matches!(self.value, TokenValue::IdentifierOrKeyword(_))
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

/// A collection of tokens.
#[derive(Debug, Default)]
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
