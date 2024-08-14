use std::cmp::PartialEq;

#[derive(Debug)]
pub enum TokenValue<'s> {
    Value(&'s str),
    Fragment(Vec<Token<'s>>),
}

#[derive(Debug)]
pub struct Token<'s> {
    value: TokenValue<'s>,
    pub start_offset: usize,
    pub end_offset: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
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
        Self { value, start_offset, end_offset, start_line, start_column, end_line, end_column }
    }

    pub fn as_str_array(&self) -> Vec<&str> {
        match &self.value {
            TokenValue::Value(value) => vec![value],
            TokenValue::Fragment(tokens) => tokens.iter().flat_map(|t| t.as_str_array()).collect(),
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
}

impl<'s, const N: usize> PartialEq<[&str; N]> for Tokens<'s> {
    fn eq(&self, other: &[&str; N]) -> bool {
        let self_str_array = self.as_str_array();
        self_str_array == other.to_vec()
    }
}
