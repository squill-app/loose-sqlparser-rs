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
    /// Arithmetic operators: `+`, `-`, `*`, `/`, `=`, `!=`, `>`, `<`, `>=`, `<=`, `<>`, `||`, `!`, `%`
    /// Bitwise operators: `~`, `&`, `|`, `<<`, `>>`, `^`
    /// PostgreSQL typecast operator: `::`
    /// Regular expression operators: `~`, `~*`, `!~`, `!~*`
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
    pub value: TokenValue<'s>,
    pub start: Position,
    pub end: Position,
}

impl<'s> Token<'s> {
    pub fn new(
        value: TokenValue<'s>,
        start_offset: usize,
        end_offset: usize,
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    ) -> Self {
        Self {
            value,
            start: Position { line: start_line, column: start_column, offset: start_offset },
            end: Position { line: end_line, column: end_column, offset: end_offset },
        }
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

#[derive(Debug)]
pub struct Tokens<'s> {
    pub tokens: Vec<Token<'s>>,
}

impl<'s> Tokens<'s> {
    pub fn as_str_array(&self) -> Vec<&str> {
        self.tokens.iter().flat_map(|t| t.as_str_array()).collect()
    }

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
