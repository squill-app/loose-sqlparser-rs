use crate::Position;
use std::convert::AsRef;
use std::ops::Index;

#[derive(Debug)]
pub enum TokenValue<'s> {
    Any(&'s str),
    Comment(&'s str),
    Quoted(&'s str),
    Delimited(&'s str),

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
            TokenValue::Quoted(value) => value,
            TokenValue::Delimited(value) => value,
            TokenValue::Operator(value) => value,
            TokenValue::StatementDelimiter(value) => value,
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

    /// The position of the first character of the token.
    pub start: Position,

    /// The position of last character of the token.
    pub end: Position,
}

impl<'s> Token<'s> {
    pub fn new(value: TokenValue<'s>, start: Position, end: Position) -> Self {
        Self { value, start, end }
    }

    pub fn is_any(&self) -> bool {
        matches!(self.value, TokenValue::Any(_))
    }

    pub fn is_comment(&self) -> bool {
        matches!(self.value, TokenValue::Comment(_))
    }

    pub fn is_quoted(&self) -> bool {
        matches!(self.value, TokenValue::Quoted(_))
    }

    pub fn is_delimited(&self) -> bool {
        matches!(self.value, TokenValue::Delimited(_))
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
            TokenValue::Quoted(value) => vec![value],
            TokenValue::Delimited(value) => vec![value],
            TokenValue::StatementDelimiter(value) => vec![value],
            TokenValue::Operator(value) => vec![value],
            TokenValue::Fragment(tokens) => tokens.tokens.iter().flat_map(|t| t.as_str_array()).collect(),
        }
    }
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            TokenValue::Fragment(tokens) => {
                for token in &tokens.tokens {
                    write!(f, "{}", token)?;
                }
                Ok(())
            }
            _ => write!(f, "{}", self.value.as_ref()),
        }
    }
}

/// A list of tokens.
#[derive(Debug)]
pub struct Tokens<'s> {
    pub tokens: Vec<Token<'s>>,
}

impl<'s> Tokens<'s> {
    /// Returns the tokens as a string array.
    /// ```rust
    /// use loose_sqlparser::loose_sqlparse;
    /// let stmt = loose_sqlparse("SELECT 1, 2").next().unwrap();
    /// let tokens = stmt.tokens();
    /// assert_eq!(tokens.as_str_array(), &["SELECT", "1", ",", "2"]);
    /// ```
    pub fn as_str_array(&self) -> Vec<&str> {
        self.tokens.iter().flat_map(|t| t.as_str_array()).collect()
    }

    /// Returns the number of token in the list.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn first(&self) -> Option<&Token<'s>> {
        self.tokens.first()
    }

    pub fn last(&self) -> Option<&Token<'s>> {
        self.tokens.last()
    }
}

/// Accessing tokens by index.
impl<'s> Index<usize> for Tokens<'s> {
    type Output = Token<'s>;

    /// Returns the token at the given index.
    ///
    /// # Panics
    /// This function panics if the index is out of bounds.
    fn index(&self, index: usize) -> &Self::Output {
        &self.tokens[index]
    }
}
