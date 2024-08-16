use crate::{Position, Statement};
use crate::{Token, TokenValue, Tokens};

// The list of all operators supported by the tokenizer.
// The tokenizer will try to match the longest operator possible, so that list must be sorted by the length descending.
const OPERATORS: [&str; 24] = [
    "!~*", "!=", ">=", "<=", "<>", "||", "<<", ">>", "::", "~*", "!~", "+", "-", "*", "/", "=", ">", "<", "!", "%",
    "~", "&", "|", "^",
];

pub(crate) struct Tokenizer<'s> {
    input: &'s str,

    // The offset of the next character to be read from the input
    // To get the current character, use `self.offset()`.
    next_offset: usize,

    // The current line of the tokenizer ()
    line: usize,

    // The current column of the tokenizer.
    column: usize,

    // The start position of the next token to be captured.
    token_start: Position,

    // The SQL delimiter used to separate statements.
    delimiter: &'s str,
}

impl<'s> Iterator for Tokenizer<'s> {
    type Item = Statement<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_offset >= self.input.len() {
            return None;
        }
        // The start of the next statement is where the tokenizer is currently positioned.
        let next = &self.input[self.next_offset..];
        let mut input_iter = next.chars();
        self.get_next_statement(input_iter.by_ref(), self.delimiter)
    }
}

impl<'s> Tokenizer<'s> {
    pub(crate) fn new(input: &'s str, delimiter: &'s str) -> Self {
        Tokenizer {
            input,
            next_offset: 0,
            line: 1,
            column: 0,
            token_start: { Position { line: 1, column: 1, offset: 0 } },
            delimiter,
        }
    }

    // The current offset of the tokenizer.
    // This is the offset of the last character read from the input.
    #[inline]
    fn offset(&self) -> usize {
        self.next_offset - 1
    }

    // The remaining input to be processed by the tokenizer, including the current character.
    #[inline]
    fn remaining_input(&self) -> &str {
        &self.input[self.offset()..]
    }

    // Handle the CRLF (Carriage Return + Line Feed) sequence.
    #[inline]
    fn process_newline(&mut self, c: char) -> bool {
        if c == '\n' {
            //
            // New Line.
            //
            self.line += 1;
            self.column = 1;
        } else if c == '\r' {
            //
            // Carriage Return (ignored).
            //
            self.column -= 1;
        } else {
            return false;
        }
        true
    }

    // Get the column number from an offset.
    // The given `offset` is expected to be on the same line as the tokenizer is currently positioned.
    #[inline]
    fn column_from_offset(&mut self, offset: usize) -> usize {
        let adjustment = offset as i64 - self.offset() as i64;
        (self.column as i64 + adjustment) as usize
    }

    // Add a token to a list of tokens.
    //
    fn add_token(
        &mut self,
        token_value: TokenValue<'s>,
        end_offset: usize,
        next_token_offset: usize,
        tokens: &mut Tokens<'s>,
    ) {
        // The `end_offset` is the offset following the last character of the token, so if `end_offset` is not equals to
        // `self.offset()`, its means the tokenizer is not currently positioned at the end of the token and `self.column`
        // cannot be used as is but must be adjusted because `self.column` is in sync with `self.offset()`.
        // The `line` does not need to be adjusted because the tokenizer is not expected to call this function when
        // positioned on a different line than the `self.line`.
        let token = Token::new(
            token_value,
            self.token_start.clone(),
            Position { line: self.line, column: self.column_from_offset(end_offset - 1), offset: end_offset },
        );
        tokens.push(token);
        self.token_start.offset = next_token_offset;
        self.token_start.line = self.line;
        self.token_start.column = self.column_from_offset(next_token_offset);
    }

    // Capture the current token.
    //
    // The token is captured from {{self.token_start_offset}} to the ending offset provided.
    // The ending offset is not included in the token.
    fn capture_token<T: Into<TokenValue<'s>>>(
        &mut self,
        tokens: &mut Tokens<'s>,
        end_offset: usize,
        next_token_offset: usize,
        value_constructor: impl Fn(&'s str) -> T,
    ) {
        if end_offset > self.token_start.offset {
            let value = value_constructor(&self.input[self.token_start.offset..end_offset]).into();
            self.add_token(value, end_offset, next_token_offset, tokens);
        } else {
            self.token_start.offset = next_token_offset;
            self.token_start.column = self.column_from_offset(next_token_offset);
        }
    }

    // Can be either `--` or `#`.
    // The `--` single-line comment is the most universally supported across different SQL dialects.
    // The `#`` single-line comment is less common and is primarily used in MySQL.
    fn capture_single_line_comment(&mut self, input_iter: &mut std::str::Chars, tokens: &mut Tokens<'s>) {
        while let Some(c) = self.get_next_char(input_iter) {
            if c == '\n' {
                // We found the end of the comment.
                self.capture_token(tokens, self.offset(), self.next_offset, TokenValue::Comment);
                self.line += 1;
                self.column = 1;
                return;
            }
        }
        // We reached the end of the input without finding the end of the comment.
        // Capture what we have so far...
        self.capture_token(tokens, self.next_offset, self.next_offset, TokenValue::Comment);
    }

    // The /* ... */ multi-line comment is widely supported supported across different SQL dialects.
    // Despite most SQL dialects not supporting nested comments, PostgreSQL does...
    // See: https://www.postgresql.org/docs/current/sql-syntax-lexical.html#SQL-SYNTAX-COMMENTS
    fn capture_multi_line_comment(&mut self, input_iter: &mut std::str::Chars, tokens: &mut Tokens<'s>) {
        // The nested level of comments (starts at 1, and decreased by 1 when a `*/` is found).
        let mut nested_level = 1;
        let mut next_char = self.get_next_char(input_iter);
        while let Some(c) = next_char {
            if c == '*' {
                next_char = self.get_next_char(input_iter);
                if next_char.as_ref() == Some(&'/') {
                    nested_level -= 1;
                    if nested_level == 0 {
                        // We found the end of the comment.
                        break;
                    }
                } else {
                    // We need to go back immediately to the beginning of the loop to check if the next character we've
                    // just read from the input.
                    continue;
                }
            } else if c == '/' {
                // We need to check if the next character is a `*` to determine if we're starting a nested comment.
                next_char = self.get_next_char(input_iter);
                if next_char.as_ref() == Some(&'*') {
                    nested_level += 1;
                } else {
                    // We need to go back immediately to the beginning of the loop to check if the next character we've
                    // just read from the input.
                    continue;
                }
            } else {
                self.process_newline(c);
            }
            next_char = self.get_next_char(input_iter);
        }
        self.capture_token(tokens, self.next_offset, self.next_offset, TokenValue::Comment);
    }

    // Capture a quoted identifier or a string literal.
    //
    // - Identifiers can be delimited by double quotes (ex: "Employee #") or backticks (`) in MySQL if he `ANSI_QUOTES`
    //   SQL mode not is enabled.
    // - String literals can be delimited by single quotes (ex: 'O''Reilly') or double quotes (ex: "O'Reilly").
    // - The quotes can be escaped by repeating the quote character, e.g., to create an identifier named
    //   'IDENTIFIER "X"', use 'IDENTIFIER ""X""'.
    //
    // Because this function has to peek the next character to check for an escaped delimiter, it returns the next
    // character to be processed by the tokenizer.
    fn capture_quoted_token(
        &mut self,
        input_iter: &mut std::str::Chars,
        quote_char: char,
        tokens: &mut Tokens<'s>,
    ) -> Option<char> {
        let mut next_char = self.get_next_char(input_iter);
        while let Some(c) = next_char {
            if c == quote_char {
                // Quote found, we need to check if it's an escaped quote (repeated quote).
                next_char = self.get_next_char(input_iter);
                if next_char.as_ref() != Some(&quote_char) {
                    // We found the end of the quoted token.
                    // We return the next character to the tokenizer so it can be processed.
                    self.capture_token(tokens, self.offset(), self.next_offset, TokenValue::Quoted);
                    return next_char;
                }
            } else {
                // Processing new line and carriage return characters in quoted identifiers is necessary because they
                // are part of the identifier.
                self.process_newline(c);
            }
            next_char = self.get_next_char(input_iter);
        }
        // We reached the end of the input without finding the end of the identifier, we still need to capture the last
        // token.
        self.capture_token(tokens, self.next_offset, self.next_offset, TokenValue::Quoted);
        next_char
    }

    #[inline]
    fn get_next_char(&mut self, input_iter: &mut std::str::Chars) -> Option<char> {
        let next_char = input_iter.next();
        if next_char.is_some() {
            self.next_offset += 1;
            self.column += 1;
        }
        next_char
    }

    // Check if the input at the current position starts with the given delimiter (case-sensitive).
    #[inline]
    fn check_delimiter(&self, delimiter: &str) -> bool {
        self.remaining_input().starts_with(delimiter)
    }

    // Move an iterator n characters forward.
    // That function is expecting that the iterator contains at least n more characters and there are no new lines
    // skipped.
    #[inline]
    fn forward_iter(&mut self, input_iter: &mut std::str::Chars, n: usize) {
        if n > 0 {
            input_iter.nth(n - 1);
            self.next_offset += n;
            self.column += n;
        }
    }

    fn capture_fragment(
        &mut self,
        input_iter: &mut std::str::Chars,
        delimiter: &str,
        tokens: &mut Tokens<'s>,
    ) -> Option<char> {
        let delimiter_start_char = delimiter.chars().next().expect("delimiter must not be empty");
        let mut next_char = self.get_next_char(input_iter);
        while let Some(c) = next_char {
            if c == '\n' {
                //
                // New Line.
                //
                self.capture_token(tokens, self.offset(), self.next_offset, TokenValue::Any);
                self.line += 1;
                self.column = 0;
                self.token_start.line = self.line;
                self.token_start.column = 1;
            } else if c == '\r' {
                //
                // Carriage Return (ignored).
                //
                self.capture_token(tokens, self.offset(), self.next_offset, TokenValue::Any);
                self.column -= 1;
            } else if c == delimiter_start_char && self.check_delimiter(delimiter) {
                //
                // Delimiter.
                //
                // Capture the last token before the delimiter and return the next character to the tokenizer so it can
                // continue the processing of the input starting from the beginning of delimiter (which is returned by
                // `next_char`).
                self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
                return next_char;
            } else if c.is_whitespace() {
                //
                // Whitespace (could be \s, \t, \r, \n, etc.).
                //
                self.capture_token(tokens, self.offset(), self.next_offset, TokenValue::Any);
            } else if c == '#' || (c == '-' && self.check_delimiter("--")) {
                //
                // Single-line comment starting by '#' (MySQL).
                // Single-line comment starting by '--' (most SQL dialects).
                //
                self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
                self.capture_single_line_comment(input_iter, tokens);
            } else if c == '/' && self.check_delimiter("/*") {
                //
                // Either a multi-line comment '/* ... */' or a division operator.
                //
                self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
                self.capture_multi_line_comment(input_iter, tokens);
            } else if c == '"' || c == '`' || c == '\'' {
                //
                // Quoted identifier or String literal.
                //
                next_char = self.capture_quoted_token(input_iter, c, tokens);
                continue;
            } else if c == '$' {
                //
                // May be dollar quoting (PostgreSQL).
                //
                // Before starting to identify the dollar-quoted delimiter we need to capture the current token.
                self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);

                // A dollar-quoted delimiter consists of a dollar sign ($), an optional “tag” of zero or more
                // characters and another dollar sign.
                // - The tag is case-sensitive, so $TAG$...$TAG$ is different from $tag$...$tag$.
                // - The tag consists of letters (A-Z, a-z), digits (0-9), and underscores (_).
                next_char = self.get_next_char(input_iter);
                while next_char.is_some()
                    && (next_char.as_ref().unwrap().is_alphanumeric() || next_char.as_ref() == Some(&'_'))
                {
                    next_char = self.get_next_char(input_iter);
                }
                if next_char.as_ref() == Some(&'$') {
                    // We found the end of the dollar-quoted delimiter.
                    let delimiter = &self.input[self.token_start.offset..self.next_offset];
                    next_char = self.capture_delimited_token(input_iter, delimiter, tokens);
                }
                continue;
            } else if c == '(' {
                //
                // Start of a parentheses block.
                //
                // Capture the previous token if any.
                self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
                // Capture the parentheses as a token.
                self.capture_token(tokens, self.next_offset, self.next_offset, TokenValue::Any);
                let mut nested_tokens = Tokens::new();
                next_char = self.capture_fragment(input_iter, delimiter, &mut nested_tokens);
                self.add_token(TokenValue::Fragment(nested_tokens), self.offset(), self.offset(), tokens);
                // We cannot assume the next character is the end of the parentheses block because we could have
                // reached the end of the input or the statement delimiter.
                if next_char.as_ref() == Some(&')') {
                    // Capturing the end parenthesis.
                    self.capture_token(tokens, self.next_offset, self.next_offset, TokenValue::Any);
                } else {
                    // End of the input or statement delimiter found.
                    return next_char;
                }
            } else if c == ')' {
                //
                // End of a parentheses block.
                //
                // Capture the last token before the end parenthesis.
                self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
                // Then we return to the caller so it can capture the end parenthesis as a token in the same fragment
                // level as the opening parenthesis.
                return next_char;
            } else if c == '.' {
                //
                // Dot (could be a decimal point or a part of an operator).
                //
                self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
                // Check if the next character is a digit to determine if the dot is part of a numeric constant.
                next_char = self.get_next_char(input_iter);
                if next_char.is_some() && next_char.as_ref().unwrap().is_ascii_digit() {
                    // The dot is part of a numeric constant.
                    next_char = self.capture_numeric_constant(input_iter, "_0123456789.eE+-", tokens);
                    continue; // `next_char` need to be processed by the tokenizer...
                } else {
                    // The dot is not part of a numeric constant, we need to capture it as a token.
                    self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
                }
            } else if c.is_numeric() {
                //
                // Numeric constant.
                //
                self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
                if c == '0' {
                    // Check the next character to determine the type of numeric constant.
                    next_char = self.get_next_char(input_iter);
                    if next_char.as_ref() == Some(&'x') || next_char.as_ref() == Some(&'X') {
                        // hexadecimal constant.
                        next_char = self.capture_numeric_constant(input_iter, "_0123456789abcdefABCDEF", tokens);
                    } else if next_char.as_ref() == Some(&'o') || next_char.as_ref() == Some(&'O') {
                        // Octal constant.
                        next_char = self.capture_numeric_constant(input_iter, "_01234567", tokens);
                    } else if next_char.as_ref() == Some(&'b') || next_char.as_ref() == Some(&'B') {
                        // Binary constant.
                        next_char = self.capture_numeric_constant(input_iter, "_01", tokens);
                    } else if next_char.is_some()
                        && (next_char.as_ref() == Some(&'.') || next_char.as_ref().unwrap().is_ascii_digit())
                    {
                        // Decimal constant.
                        next_char = self.capture_numeric_constant(input_iter, "_0123456789.eE+-", tokens);
                    } else {
                        // We found a single zero, we need to capture it as a numeric constant.
                        self.capture_token(tokens, self.offset(), self.offset(), TokenValue::NumericConstant);
                    }
                } else {
                    next_char = self.capture_numeric_constant(input_iter, "_0123456789.eE+-", tokens);
                }
                continue; // `next_char` need to be processed by the tokenizer...
            } else if !c.is_alphabetic() && c != '_' {
                //
                // Any other character that is not an underscore or alphabetic will be considered as a boundary
                // for a token, except for operators.
                //
                if !self.try_capture_operator(input_iter, tokens) {
                    self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
                }
            }
            next_char = self.get_next_char(input_iter);
        }

        // The delimiter was not found and we reached the end of the input, we need to capture the last token.
        self.capture_token(tokens, self.next_offset, self.offset(), TokenValue::Any);
        next_char
    }

    // Try to capture an operator.
    //
    // The tokenizer will try to match the longest operator possible.
    // If an operator is found:
    // - 2 tokens will be added to the tokens list if the operator is preceded by a token, otherwise only the operator
    //   token will be added.
    // - the iterator will be moved to the end of the operator.
    //
    // Returns true if an operator was found, false otherwise.
    fn try_capture_operator(&mut self, input_iter: &mut std::str::Chars, tokens: &mut Tokens<'s>) -> bool {
        let remaining_input = &self.input[self.offset()..];
        let operator = OPERATORS.iter().find(|&op| remaining_input.starts_with(op));
        if let Some(op) = operator {
            // We found an operator, we need to capture the current token before the operator.
            self.capture_token(tokens, self.offset(), self.offset(), TokenValue::Any);
            // Capture the operator
            self.capture_token(tokens, self.offset() + op.len(), self.offset() + op.len(), TokenValue::Operator);
            // We need to move the iterator to the end of the operator.
            self.forward_iter(input_iter, op.len() - 1);
            true
        } else {
            false
        }
    }

    // Capture a token delimited by the given delimiter.
    //
    // The delimiter can be a single character or a multi-character delimiter.
    // There is no escaping mechanism for delimiters, so if the delimiter is found the token is captured and the next
    // character is returned to the tokenizer.
    //
    // This is used to capture Dollar-Quoted Strings in PostgreSQL.
    // See: https://www.postgresql.org/docs/current/sql-syntax-lexical.html#SQL-SYNTAX-DOLLAR-QUOTING
    //
    // Also used to capture more complex SQL constructs like JSON literals in MySQL when using the 'DELIMITER' cli
    // command.
    // See: https://dev.mysql.com/doc/refman/8.4/en/stored-programs-defining.html
    //
    // This function will panic if the delimiter is an empty string.
    fn capture_delimited_token(
        &mut self,
        input_iter: &mut std::str::Chars,
        delimiter: &str,
        tokens: &mut Tokens<'s>,
    ) -> Option<char> {
        let delimiter_start_char = delimiter.chars().next().expect("delimiter must not be empty");
        let mut next_char = self.get_next_char(input_iter);
        while let Some(c) = next_char {
            if c == delimiter_start_char {
                // We've found the first character of the delimiter, let check if we have the full delimiter in the
                // input.
                let remaining_input = &self.input[self.offset()..];
                if remaining_input.starts_with(delimiter) {
                    // We found the end of the delimited token.
                    self.capture_token(
                        tokens,
                        self.offset() + delimiter.len(),
                        self.offset() + delimiter.len(),
                        TokenValue::Delimited,
                    );
                    // We return the next character to the tokenizer so it can be processed.
                    self.forward_iter(input_iter, delimiter.len() - 1);
                    return self.get_next_char(input_iter);
                }
            } else {
                // Processing new line and carriage return characters in quoted identifiers is necessary because they
                // are part of the identifier.
                self.process_newline(c);
            }
            next_char = self.get_next_char(input_iter);
        }
        // We reached the end of the input without finding the end of the token...
        next_char
    }

    // Get the next statement from the input.
    // The end of the next statement is determined by the delimiter provided or the end of the input.
    fn get_next_statement(&mut self, input_iter: &mut std::str::Chars, delimiter: &str) -> Option<Statement<'s>> {
        // Capture all tokens until the next semicolon.
        let mut tokens = Tokens::new();

        // Under normal circumstances, the tokenizer will either return None if the input is empty or the first
        // character if the delimiter if found.
        // Nevertheless we need to handle the case where the tokenizer was stopped by a closing parenthesis without a
        // matching opening parenthesis. This is why we need to loop until we find the delimiter or reach the end of the
        // input.
        while self.capture_fragment(input_iter, delimiter, &mut tokens).is_some() {
            if self.check_delimiter(delimiter) {
                // The delimiter was found but not captured as a token, we need to capture it now.
                // Moving forward the iterator until the end of the delimiter.
                self.forward_iter(input_iter, delimiter.len() - 1);
                self.capture_token(&mut tokens, self.next_offset, self.next_offset, TokenValue::StatementDelimiter);
                break;
            } else {
                // We need to continue the tokenization because we found a closing parenthesis without a matching
                // opening parenthesis.
                // We need to capture the closing parenthesis as a token before resuming the tokenization.
                self.capture_token(&mut tokens, self.next_offset, self.next_offset, TokenValue::Any);
            }
        }

        match tokens.is_empty() {
            // We reached the end of the input without finding any token.
            true => None,
            false => Some(Statement { input: self.input, tokens }),
        }
    }

    // Capture a Numeric Constants
    //
    // The numeric constant will be captured until we reach any character that is not in the provided `allowed_chars`.
    // `+` and `-` are allowed only if the previous character is `e` (exponential notation: `digits.[digits][e[+-]digits]`),
    // a leading `+` or `-` is not captured as a sign of the numeric constant but as an operator.
    fn capture_numeric_constant(
        &mut self,
        input_iter: &mut std::str::Chars,
        allowed_chars: &str,
        tokens: &mut Tokens<'s>,
    ) -> Option<char> {
        let mut next_char = self.get_next_char(input_iter);
        while let Some(c) = next_char {
            if !allowed_chars.contains(c) {
                break;
            } else if c == 'e' || c == 'E' {
                // Check if the next character is a sign (+ or -) or a digit to allow exponential notation.
                next_char = self.get_next_char(input_iter);
                if next_char.is_none()
                    || (next_char.as_ref() != Some(&'+')
                        && next_char.as_ref() != Some(&'-')
                        && !next_char.as_ref().unwrap().is_ascii_digit())
                {
                    break;
                }
            } else if c == '+' || c == '-' {
                // Only allowed as the sign of the exponent which should have been captured by the previous condition.
                break;
            }
            next_char = self.get_next_char(input_iter);
        }

        // We reached the end of the numeric constant or the end of the input.
        let end_offset = if next_char.is_some() { self.offset() } else { self.next_offset };
        self.capture_token(tokens, end_offset, end_offset, TokenValue::NumericConstant);
        next_char
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_constant() {
        let expected: Vec<&str> = vec![
            "0",
            "1",
            "42",
            "3.5",
            "4.",
            ".001",
            "5e2",
            "1.925e-3",
            "0b100101",
            "0B10011001",
            "0o273",
            "0O755",
            "0x42f",
            "0XFFFF",
            "1_500_000_000",
            "0b10001000_00000000",
            "0o_1_755",
            "0xFFFF_FFFF",
            "1.618_034",
        ];
        let expected_string = expected.join(" ");
        let s: Vec<_> = Tokenizer::new(&expected_string, ";").collect();
        let tokens = s[0].tokens();
        assert_eq!(tokens.as_str_array(), expected);
        for token in tokens.iter() {
            assert!(token.is_numeric_constant());
        }

        // The tokenizer should not capture the +/- as part of the numeric constant if not part of the exponential
        // notation.
        let s: Vec<_> = Tokenizer::new("1.925e-3+1 1.618_034+1 0b100101+1 0o273+1 0x42f+1", ";").collect();
        assert_eq!(
            s[0].tokens().as_str_array(),
            ["1.925e-3", "+", "1", "1.618_034", "+", "1", "0b100101", "+", "1", "0o273", "+", "1", "0x42f", "+", "1"]
        );
    }

    #[test]
    fn test_comma() {
        let s: Vec<_> = Tokenizer::new("1, 2, /* , */", ";").collect();
        let tokens = s[0].tokens();
        assert_eq!(tokens.as_str_array(), ["1", ",", "2", ",", "/* , */"]);
        assert!(tokens[1].is_comma());
        assert!(tokens[3].is_comma());
    }

    #[test]
    fn test_operators() {
        let s: Vec<_> = Tokenizer::new("1 + 2+3 -4-5 * 6*7 / 8/9", ";").collect();
        let tokens = s[0].tokens();
        assert_eq!(
            tokens.as_str_array(),
            ["1", "+", "2", "+", "3", "-", "4", "-", "5", "*", "6", "*", "7", "/", "8", "/", "9"]
        );
    }

    #[test]
    fn test_parenthesis() {
        // A missing opening parenthesis should not stop the tokenizer when reaching a closing parenthesis.
        let s: Vec<_> = Tokenizer::new("SELECT 1 + 2) + 3; SELECT 2", ";").collect();
        assert!(s.len() == 2);
        assert_eq!(s[0].tokens().as_str_array(), ["SELECT", "1", "+", "2", ")", "+", "3", ";"]);
        assert!(s[0].tokens().last().unwrap().is_statement_delimiter());
        assert_eq!(s[1].tokens().as_str_array(), ["SELECT", "2"]);

        let s: Vec<_> = Tokenizer::new("SELECT (1 + 2) * 3", ";").collect();
        let tokens = s[0].tokens();
        assert_eq!(tokens.as_str_array(), ["SELECT", "(", "1", "+", "2", ")", "*", "3"]);
        assert!(tokens[1].is_parenthesis());
        assert!(tokens[2].is_fragment());
        assert!(tokens[3].is_parenthesis());

        let s: Vec<_> = Tokenizer::new("SELECT ((1+2)*(3*4))", ";").collect();
        let tokens = s[0].tokens();
        assert_eq!(tokens.as_str_array(), ["SELECT", "(", "(", "1", "+", "2", ")", "*", "(", "3", "*", "4", ")", ")"]);

        // A missing closing parenthesis should not prevent the tokenizer to stop when reaching the statement delimiter.
        let s: Vec<_> = Tokenizer::new("SELECT (1 + 2 + 3; SELECT 2", ";").collect();
        assert!(s.len() == 2);
        assert_eq!(s[0].tokens().as_str_array(), ["SELECT", "(", "1", "+", "2", "+", "3", ";"]);
        assert!(s[0].tokens().last().unwrap().is_statement_delimiter());
        assert_eq!(s[1].tokens().as_str_array(), ["SELECT", "2"]);
    }

    #[test]
    fn test_delimited_token() {
        let s: Vec<_> = Tokenizer::new("BEGIN $$O'Reilly$$, $tag$with_tag$tag$, $x$__$__$x$ END", ";").collect();
        let tokens = s[0].tokens();
        assert_eq!(
            tokens.as_str_array(),
            ["BEGIN", "$$O'Reilly$$", ",", "$tag$with_tag$tag$", ",", "$x$__$__$x$", "END"]
        );
        assert!(tokens[1].is_delimited());
        assert!(tokens[3].is_delimited());
        assert!(tokens[5].is_delimited());
    }

    #[test]
    fn test_multi_line_comment() {
        let s: Vec<_> =
            Tokenizer::new("/* /*nested*/comment */ /** line\n *  break\n **/ /* not closed...", ";").collect();
        let tokens = s[0].tokens();
        assert_eq!(tokens.as_str_array(), ["/* /*nested*/comment */", "/** line\n *  break\n **/", "/* not closed..."]);
        assert!(tokens[0].is_comment());
        assert!(tokens[1].is_comment());
        assert!(tokens[2].is_comment());
    }

    #[test]
    fn test_single_line_comment() {
        let s: Vec<_> = Tokenizer::new("-- comment\n# comment\n# comment", ";").collect();
        let tokens = s[0].tokens();
        assert_eq!(tokens.as_str_array(), ["-- comment", "# comment", "# comment"]);
        assert!(tokens[0].is_comment());
        assert!(tokens[1].is_comment());
        assert!(tokens[2].is_comment());
    }

    #[test]
    fn test_quoted_token() {
        let s: Vec<_> = Tokenizer::new(r#"'' "ID" "ID ""X""" '''' 'O''Reilly' "#, ";").collect();
        let tokens = s[0].tokens();
        assert_eq!(tokens.as_str_array(), ["''", r#""ID""#, r#""ID ""X""""#, "''''", "'O''Reilly'"]);
        assert!(tokens[1].is_quoted());
        assert!(tokens[2].is_quoted());
    }

    #[test]
    fn test_split_statements() {
        let s: Vec<_> = Tokenizer::new("SELECT 1; SELECT 2", ";").collect();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].sql(), "SELECT 1;");
        assert_eq!(s[1].sql(), "SELECT 2");
    }

    #[test]
    fn test_empty_input() {
        let s: Vec<_> = Tokenizer::new("", ";").collect();
        assert_eq!(s.len(), 0);
    }
}
