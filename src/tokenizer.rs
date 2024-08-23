use crate::{Options, Position, Statement};
use crate::{Token, TokenValue, Tokens};

// The list of all operators supported by the tokenizer.
// The tokenizer will try to match the longest operator possible, so that list must be sorted by the length descending.
const OPERATORS: [&str; 24] = [
    "!~*", "!=", ">=", "<=", "<>", "||", "<<", ">>", "::", "~*", "!~", "+", "-", "*", "/", "=", ">", "<", "!", "%",
    "~", "&", "|", "^",
];

pub(crate) struct Tokenizer<'s> {
    // The input to be tokenized (the whole SQL to be tokenized).
    input: &'s str,

    // The byte offset of the current character in the input.
    offset: usize,

    // The byte offset of the next character in the input
    next_offset: usize,

    // The current line of the tokenizer ()
    line: usize,

    // The current column of the tokenizer.
    column: usize,

    // The start position of the next token to be captured.
    token_start: Position,

    // The tokenizer options.
    options: Options,
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
        self.get_next_statement(input_iter.by_ref(), &self.options.statement_delimiter.clone())
    }
}

impl<'s> Tokenizer<'s> {
    pub(crate) fn new(input: &'s str, options: Options) -> Self {
        Tokenizer {
            input,
            options,
            offset: 0,
            next_offset: 0,
            line: 1,
            column: 0,
            token_start: { Position { line: 1, column: 1, offset: 0 } },
        }
    }

    // Extract the next character from the given iterator.
    #[inline]
    fn get_next_char(&mut self, input_iter: &mut std::str::Chars) -> Option<char> {
        let next_char = input_iter.next();
        if next_char.is_some() {
            self.offset = self.next_offset;
            self.next_offset += next_char.as_ref().unwrap().len_utf8();
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
    #[inline]
    fn forward_iter(&mut self, input_iter: &mut std::str::Chars, n: usize) {
        let mut n = n;
        while n > 0 && self.get_next_char(input_iter).is_some() {
            n -= 1;
        }
    }

    // The remaining input to be processed by the tokenizer, including the current character.
    #[inline]
    fn remaining_input(&self) -> &str {
        &self.input[self.offset..]
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
    //
    // WARNING: This function is not safe to use if the given `offset` is not on the same line as the current position
    // (`self.offset` of the tokenizer.
    #[inline]
    fn column_from_offset(&mut self, offset: usize) -> usize {
        if offset == self.offset {
            self.column
        } else {
            // Because strings can contain multi-byte characters, we need to count the number of characters between the
            // current position and the given offset.
            let str = &self.input[std::cmp::min(self.offset, offset)..std::cmp::max(offset, self.offset)];
            let count = match offset.cmp(&self.offset) {
                std::cmp::Ordering::Less => -1,
                _ => 1,
            } * str.chars().count() as i64;
            (self.column as i64 + count) as usize
        }
    }

    // Add a token to a list of tokens.
    //
    // The `end_offset` designated the position of the character immediately following the token. Which means the token
    // is captured from `self.token_start.offset` to `end_offset - 1`.
    fn add_token(
        &mut self,
        token_value: TokenValue<'s>,
        end_offset: usize,
        next_token_offset: usize,
        tokens: &mut Tokens<'s>,
    ) {
        // The `end_offset` is the offset following the last character of the token, so if `end_offset` is not equals to
        // `self.offset`, its means the tokenizer is not currently positioned at the end of the token and `self.column`
        // cannot be used as is and must be adjusted because `self.column` is in sync with `self.offset`.
        // The `line` does not need to be adjusted because the tokenizer is not expected to call this function when
        // positioned on a different line than the `self.line`.
        let token = Token::new(
            token_value,
            self.token_start.clone(),
            Position { line: self.line, column: self.column_from_offset(end_offset) - 1, offset: end_offset },
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
                self.capture_token(tokens, self.offset, self.next_offset, TokenValue::Comment);
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
                    // back to the main loop to process the character we've just read from the input.
                    continue;
                }
            } else if c == '/' {
                // We need to check if the next character is a `*` to determine if we're starting a nested comment.
                next_char = self.get_next_char(input_iter);
                if next_char.as_ref() == Some(&'*') {
                    nested_level += 1;
                } else {
                    // back to the main loop to process the character we've just read from the input.
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
    fn capture_quoted_identifier_or_constant(
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
                    // We found the end of the quoted token (or the end of the input).
                    // We return the next character to the tokenizer so it can be processed.
                    self.capture_token(
                        tokens,
                        if next_char.is_some() { self.offset } else { self.next_offset },
                        if next_char.is_some() { self.offset } else { self.next_offset },
                        TokenValue::QuotedIdentifierOrConstant,
                    );
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
        self.capture_token(tokens, self.next_offset, self.next_offset, TokenValue::QuotedIdentifierOrConstant);
        next_char
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
                self.capture_token(tokens, self.offset, self.next_offset, TokenValue::Any);
                self.line += 1;
                self.column = 0;
                self.token_start.line = self.line;
                self.token_start.column = 1;
            } else if c == '\r' {
                //
                // Carriage Return (ignored).
                //
                self.capture_token(tokens, self.offset, self.next_offset, TokenValue::Any);
                self.column -= 1;
            } else if c == delimiter_start_char && self.check_delimiter(delimiter) {
                //
                // Delimiter.
                //
                // Capture the last token before the delimiter and return the next character to the tokenizer so it can
                // continue the processing of the input starting from the beginning of delimiter (which is returned by
                // `next_char`).
                self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
                return next_char;
            } else if c.is_whitespace() {
                //
                // Whitespace (could be \s, \t, \r, \n, etc.).
                //
                self.capture_token(tokens, self.offset, self.next_offset, TokenValue::Any);
            } else if c == '#' || (c == '-' && self.check_delimiter("--")) {
                //
                // Single-line comment starting by '#' (MySQL).
                // Single-line comment starting by '--' (most SQL dialects).
                //
                self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
                self.capture_single_line_comment(input_iter, tokens);
            } else if c == '/' && self.check_delimiter("/*") {
                //
                // Either a multi-line comment '/* ... */' or a division operator.
                //
                self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
                self.capture_multi_line_comment(input_iter, tokens);
            } else if c == '\'' || c == '"' || c == '`' {
                //
                // Quoted identifier or constant.
                //
                if c == '\'' && self.offset > self.token_start.offset {
                    // There is an introducer:
                    // - Escaped string constant (E'hello\\tworld').
                    // - Unicode string constant (N'こんにちは').
                    // - Bit-String constant (B'1001', X'1FF').
                    // - String constant with a character set introducer (_latin1'hello').
                    let introducer = &self.input[self.token_start.offset..self.offset];
                    let first_char = introducer.chars().next().unwrap();
                    if first_char == 'B' || first_char == 'b' || first_char == 'X' || first_char == 'x' {
                        // Escaped quotes are not allowed by Bit-String constants.
                        next_char = self.capture_delimited_token(
                            input_iter,
                            &c.to_string(),
                            tokens,
                            TokenValue::QuotedIdentifierOrConstant,
                        );
                        continue;
                    }
                }
                next_char = self.capture_quoted_identifier_or_constant(input_iter, c, tokens);
                continue;
            } else if (c == 'U' || c == 'u') && self.remaining_input().starts_with("U&\"") {
                //
                // Escaped Unicode quoted identifier (PostgreSQL: U&"d\0061t\+000061").
                //
                // A Unicode escape string constant starts with U& (upper or lower case letter U followed by ampersand)
                // immediately before the opening quote, without any spaces in between, for example U&"foo".
                self.forward_iter(input_iter, 2);
                next_char = self.capture_quoted_identifier_or_constant(input_iter, '"', tokens);
                continue;
            } else if c == '$' {
                //
                // May be dollar quoting (PostgreSQL).
                //
                // Before starting to identify the dollar-quoted delimiter we need to capture the current token.
                self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);

                // A dollar-quoted delimiter consists of a dollar sign ($), an optional “tag” of zero or more
                // characters and another dollar sign.
                // - The tag is case-sensitive, so $TAG$...$TAG$ is different from $tag$...$tag$.
                // - The tag consists of letters (A-Z, a-z), digits (0-9), and underscores (_).
                next_char = self.get_next_char(input_iter);
                while next_char.is_some()
                    && (next_char.as_ref().unwrap().is_ascii_alphanumeric() || next_char.as_ref() == Some(&'_'))
                {
                    next_char = self.get_next_char(input_iter);
                }
                if next_char.as_ref() == Some(&'$') {
                    // We found the end of the dollar-quoted delimiter.
                    let delimiter = &self.input[self.token_start.offset..self.next_offset];
                    next_char = self.capture_delimited_token(
                        input_iter,
                        delimiter,
                        tokens,
                        TokenValue::QuotedIdentifierOrConstant,
                    );
                } else {
                    // We've found a parameter marker (`$1`, `$id`)
                    self.capture_token(
                        tokens,
                        if next_char.is_some() { self.offset } else { self.next_offset },
                        self.next_offset,
                        TokenValue::ParameterMarker,
                    );
                }
                continue;
            } else if c == ':' || c == '?' || c == '@' {
                //
                // A Parameter Marker
                //
                next_char = self.get_next_char(input_iter);
                while next_char.is_some()
                    && (next_char.as_ref().unwrap().is_ascii_alphanumeric() || next_char.as_ref() == Some(&'_'))
                {
                    next_char = self.get_next_char(input_iter);
                }
                if c == ':' && next_char.as_ref() == Some(&':') && self.token_start.offset + 1 == self.offset {
                    // Special case for the PostgreSQL type casting operator `::` (consuming next_char).
                    self.capture_token(tokens, self.next_offset, self.next_offset, TokenValue::Operator);
                } else {
                    // We've found a parameter marker (`$1`, `$id`)
                    self.capture_token(
                        tokens,
                        if next_char.is_some() { self.offset } else { self.next_offset },
                        self.next_offset,
                        TokenValue::ParameterMarker,
                    );
                    continue;
                }
            } else if c == '(' {
                //
                // Start of a parentheses block.
                //
                // Capture the previous token if any.
                self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
                // Capture the parentheses as a token.
                self.capture_token(tokens, self.next_offset, self.next_offset, TokenValue::Any);
                let mut nested_tokens = Tokens::new();
                next_char = self.capture_fragment(input_iter, delimiter, &mut nested_tokens);
                self.add_token(TokenValue::Fragment(nested_tokens), self.offset, self.offset, tokens);
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
                self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
                // Then we return to the caller so it can capture the end parenthesis as a token in the same fragment
                // level as the opening parenthesis.
                return next_char;
            } else if c == '.' {
                //
                // Dot (start of a decimal constant (ex: .05), or part of a qualified name (ex: schema.table)).
                //
                self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
                // Check if the next character is a digit to determine if the dot is part of a numeric constant.
                next_char = self.get_next_char(input_iter);
                if next_char.is_some() && next_char.as_ref().unwrap().is_ascii_digit() {
                    // The dot is part of a numeric constant.
                    next_char = self.capture_numeric_constant(input_iter, "_0123456789.eE+-", tokens);
                } else {
                    // The dot is not part of a numeric constant, we need to capture it as a token.
                    self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
                }
                continue; // `next_char` need to be processed by the tokenizer...
            } else if c.is_numeric() {
                //
                // Numeric constant.
                //
                self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
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
                    } else if next_char.is_some() {
                        // We found a single zero ('0') followed by a character that is not part of a numeric constant.
                        self.capture_token(tokens, self.offset, self.offset, TokenValue::NumericConstant);
                    } else {
                        // We found a single zero ('0') a the end of the input.
                        self.capture_token(tokens, self.offset, self.next_offset, TokenValue::NumericConstant);
                    }
                } else {
                    next_char = self.capture_numeric_constant(input_iter, "_0123456789.eE+-", tokens);
                }
                continue; // `next_char` need to be processed by the tokenizer...
            } else if c.is_alphabetic() || c == '_' {
                //
                // Identifier or keyword.
                //
                self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
                next_char = self.try_capture_identifier_or_keyword(input_iter, tokens);
                continue; // `next_char` need to be processed by the tokenizer...
            } else {
                //
                // Any other character that is not an underscore or alphabetic will be considered as a boundary
                // for a token, except for operators.
                //
                if !self.try_capture_operator(input_iter, tokens) {
                    self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
                }
            }
            next_char = self.get_next_char(input_iter);
        }

        // The delimiter was not found and we reached the end of the input, we need to capture the last token.
        self.capture_token(tokens, self.next_offset, self.offset, TokenValue::Any);
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
        let remaining_input = &self.input[self.offset..];
        let operator = OPERATORS.iter().find(|&op| remaining_input.starts_with(op));
        if let Some(op) = operator {
            // We found an operator, we need to capture the current token before the operator.
            self.capture_token(tokens, self.offset, self.offset, TokenValue::Any);
            // Capture the operator
            self.capture_token(tokens, self.offset + op.len(), self.offset + op.len(), TokenValue::Operator);
            // We need to move the iterator to the end of the operator.
            self.forward_iter(input_iter, op.chars().count() - 1);
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
    // - Used to capture Dollar-Quoted Strings in PostgreSQL.
    // See: https://www.postgresql.org/docs/current/sql-syntax-lexical.html#SQL-SYNTAX-DOLLAR-QUOTING
    //
    // Also used to capture more complex SQL constructs like JSON literals in MySQL when using the 'DELIMITER' cli
    // command.
    // See: https://dev.mysql.com/doc/refman/8.4/en/stored-programs-defining.html
    //
    // This function will panic if the delimiter is an empty string.
    fn capture_delimited_token<T: Into<TokenValue<'s>>>(
        &mut self,
        input_iter: &mut std::str::Chars,
        delimiter: &str,
        tokens: &mut Tokens<'s>,
        value_constructor: impl Fn(&'s str) -> T,
    ) -> Option<char> {
        let delimiter_start_char = delimiter.chars().next().expect("delimiter must not be empty");
        let mut next_char = self.get_next_char(input_iter);
        while let Some(c) = next_char {
            if c == delimiter_start_char {
                // We've found the first character of the delimiter, let check if we have the full delimiter in the
                // input.
                let remaining_input = &self.input[self.offset..];
                if remaining_input.starts_with(delimiter) {
                    // We found the end of the delimited token.
                    self.capture_token(
                        tokens,
                        self.offset + delimiter.len(),
                        self.offset + delimiter.len(),
                        value_constructor,
                    );
                    // We return the next character to the tokenizer so it can be processed.
                    self.forward_iter(input_iter, delimiter.chars().count() - 1);
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
        self.capture_token(tokens, self.next_offset, self.next_offset, value_constructor);
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
                self.forward_iter(input_iter, delimiter.chars().count() - 1);
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
        let end_offset = if next_char.is_some() { self.offset } else { self.next_offset };
        self.capture_token(tokens, end_offset, end_offset, TokenValue::NumericConstant);
        next_char
    }

    /// Try to capture an identifier or a keyword.
    ///
    /// SQL identifiers and key words must begin with a letter (a-z, but also letters with diacritical marks and
    /// non-Latin letters) or an underscore (_). Subsequent characters in an identifier or key word can be letters,
    /// underscores, digits (0-9), or dollar signs ($).
    ///
    /// If the token is immediately followed by a single quote (') or a double quote (") it will not be captured because
    /// it should be captured as a part of a constant with a introducer (E'', N'', _latin1'', ...).
    fn try_capture_identifier_or_keyword(
        &mut self,
        input_iter: &mut std::str::Chars,
        tokens: &mut Tokens<'s>,
    ) -> Option<char> {
        let mut next_char = self.get_next_char(input_iter);
        while let Some(c) = next_char {
            if c.is_alphanumeric() || c == '_' || c == '$' {
                next_char = self.get_next_char(input_iter);
            } else {
                break;
            }
        }
        if next_char.as_ref() == Some(&'\'') {
            // The identifier or keyword is followed by a quote, it should be captured as a constant with an introducer.
            return next_char;
        }
        // We reached the end of the identifier or keyword (or the end of the input).
        let end_offset = if next_char.is_some() { self.offset } else { self.next_offset };
        self.capture_token(tokens, end_offset, end_offset, TokenValue::IdentifierOrKeyword);
        next_char
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A macro that check if the input is captured as a token of the given variant, expected positions and value.
    //
    // The value is duplicated in the input to make sure the tokenizer works when the token is followed by a space
    // and when the token is the last token of the input (i.e., "input input").
    //
    // IMPORTANT: The input must not contain a Carriage Return.
    macro_rules! assert_token {
        ($input:expr, $token_variant:ident) => {
            let input = format!("{} {}", $input, $input);
            let statement = Tokenizer::new(&input, Options::default()).next();
            assert!(statement.is_some());
            let tokens = statement.as_ref().unwrap().tokens();
            for (index, token) in tokens.iter().enumerate() {
                let expected_start_column = index * $input.chars().count() + (index + 1);
                let expected_end_column = index * $input.chars().count() + $input.chars().count() + index;
                let expected_offset = index * $input.len() + index;
                assert!(matches!(token.value, TokenValue::$token_variant(_)), "Variant mismatch: {:?}", token);
                assert_eq!(token.value.as_ref(), $input);
                assert_eq!(token.start.column, expected_start_column, "`start.column` mismatch: {:?}", token);
                assert_eq!(token.end.column, expected_end_column, "`end.column` mismatch: {:?}", token);
                assert_eq!(token.start.offset, expected_offset, "`offset` mismatch: {:?}", token);
            }
        };
    }

    macro_rules! assert_tokens {
        ($input:expr, $( $expected:expr ),* ) => {
            let mut statements = Tokenizer::new($input, Options::default()).into_iter();
            let expected_values = vec![$( $expected.as_slice() ),*];
            for expected in expected_values {
                let statement = statements.next();
                assert!(statement.is_some(), "Expected more statements.");
                assert_eq!(
                    statement.as_ref().unwrap().tokens().as_str_array(),
                    expected,
                    "Tokens do not match the expected values."
                );
            }

            assert!(statements.next().is_none(), "Expected no more statements.");
        };
    }

    #[test]
    fn test_quoted_identifier_with_unicode_escapes() {
        assert_token!(r#"U&"d\\0061t\\+000061""#, QuotedIdentifierOrConstant);
        assert_token!(r#"U&"\\0441\\043B\\043E\\043D""#, QuotedIdentifierOrConstant);
    }

    #[test]
    fn test_escaped_or_unicode_string_constant() {
        assert_token!("E''", QuotedIdentifierOrConstant);
        assert_token!("E'hello\\world'", QuotedIdentifierOrConstant);
        assert_token!("e''", QuotedIdentifierOrConstant);
        assert_token!("e'hello\\world'", QuotedIdentifierOrConstant);
        assert_token!("N''", QuotedIdentifierOrConstant);
        assert_token!("N'こんにちは'", QuotedIdentifierOrConstant);
        assert_token!("n''", QuotedIdentifierOrConstant);
        assert_token!("n'こんにちは'", QuotedIdentifierOrConstant);
    }

    #[test]
    fn test_bit_string_constant() {
        assert_token!("B'100'", QuotedIdentifierOrConstant);
        assert_token!("B''", QuotedIdentifierOrConstant);
        assert_token!("b'100'", QuotedIdentifierOrConstant);
        assert_token!("b''", QuotedIdentifierOrConstant);
        assert_token!("x'1FF'", QuotedIdentifierOrConstant);
        assert_token!("x''", QuotedIdentifierOrConstant);
    }

    #[test]
    fn test_string_constant_with_charset_introducer() {
        // A character string literal may have an optional character set introducer (MySQL).
        // https://dev.mysql.com/doc/refman/8.4/en/string-literals.html
        assert_token!("_latin1'string'", QuotedIdentifierOrConstant);
        assert_token!("_latin1''", QuotedIdentifierOrConstant);
        assert_token!("_binary'string'", QuotedIdentifierOrConstant);
        assert_token!("_utf8mb4'string'", QuotedIdentifierOrConstant);
    }

    #[test]
    fn test_capture_identifier_or_keyword_token() {
        assert_token!("column", IdentifierOrKeyword);
        assert_token!("column1", IdentifierOrKeyword);
        assert_token!("column_", IdentifierOrKeyword);
        assert_token!("column_name", IdentifierOrKeyword);
        assert_token!("column$", IdentifierOrKeyword);
        assert_token!("column$name", IdentifierOrKeyword);
        assert_token!("ColumnName", IdentifierOrKeyword);
        assert_token!("naïve_table", IdentifierOrKeyword);
        assert_token!("_leading_underscore", IdentifierOrKeyword);
        assert_token!("trailing_underscore_", IdentifierOrKeyword);
        assert_token!("_leading_and_trailing_underscore_", IdentifierOrKeyword);
        assert_token!("__double__underscores__", IdentifierOrKeyword);
        assert_token!("_$$", IdentifierOrKeyword);
    }

    #[test]
    fn test_numeric_constant_token() {
        assert_token!("0", NumericConstant);
        assert_token!("0.", NumericConstant);
        assert_token!("1", NumericConstant);
        assert_token!("42", NumericConstant);
        assert_token!("3.5", NumericConstant);
        assert_token!("4.", NumericConstant);
        assert_token!(".001", NumericConstant);
        assert_token!("5e2", NumericConstant);
        assert_token!("1.925e-3", NumericConstant);
        assert_token!("0b100101", NumericConstant);
        assert_token!("0B10011001", NumericConstant);
        assert_token!("0o273", NumericConstant);
        assert_token!("0O755", NumericConstant);
        assert_token!("0x42f", NumericConstant);
        assert_token!("0XFFFF", NumericConstant);
        assert_token!("1_500_000_000", NumericConstant);
        assert_token!("0b10001000_00000000", NumericConstant);
        assert_token!("0o_1_755", NumericConstant);
        assert_token!("0xFFFF_FFFF", NumericConstant);
        assert_token!("1.618_034", NumericConstant);

        // Should not capture the +/- as part of the numeric constant if not part of the exponential notation.
        assert_tokens!("1.925e-3+1 1.925-3 1.925+3", ["1.925e-3", "+", "1", "1.925", "-", "3", "1.925", "+", "3"]);

        // Should break invalid numeric constants.
        assert_tokens!("0xg", ["0x", "g"]);
        assert_tokens!("1.9eg", ["1.9e", "g"]);
    }

    #[test]
    fn test_comma() {
        assert_tokens!("1, 2, /* , */", ["1", ",", "2", ",", "/* , */"]);
    }

    #[test]
    fn test_parameter_marker_token() {
        assert_token!("?", ParameterMarker);
        assert_token!("$1", ParameterMarker);
        assert_token!(":username", ParameterMarker);
        assert_token!("$username", ParameterMarker);
        assert_token!("@username", ParameterMarker);
        assert_tokens!("id = ? AND name = ?", ["id", "=", "?", "AND", "name", "=", "?"]);
        assert_tokens!(
            "id = ? AND name = '_prefix'||?||'_suffix'",
            ["id", "=", "?", "AND", "name", "=", "'_prefix'", "||", "?", "||", "'_suffix'"]
        );
        assert_tokens!("id = $1 AND name = $2", ["id", "=", "$1", "AND", "name", "=", "$2"]);
        assert_tokens!(
            "id = :user_id AND name = :user_name",
            ["id", "=", ":user_id", "AND", "name", "=", ":user_name"]
        );
        assert_tokens!(
            "id = @user_id AND name = @user_name",
            ["id", "=", "@user_id", "AND", "name", "=", "@user_name"]
        );
        assert_tokens!(
            "id = $user_id AND name = $user_name",
            ["id", "=", "$user_id", "AND", "name", "=", "$user_name"]
        );
    }

    #[test]
    fn test_operator_token() {
        assert_token!("!~*", Operator);
        assert_token!("!=", Operator);
        assert_token!(">=", Operator);
        assert_token!("<=", Operator);
        assert_token!("<>", Operator);
        assert_token!("||", Operator);
        assert_token!("<<", Operator);
        assert_token!(">>", Operator);
        assert_token!("::", Operator);
        assert_token!("~*", Operator);
        assert_token!("!~", Operator);
        assert_token!("+", Operator);
        assert_token!("-", Operator);
        assert_token!("*", Operator);
        assert_token!("/", Operator);
        assert_token!("=", Operator);
        assert_token!(">", Operator);
        assert_token!("<", Operator);
        assert_token!("!", Operator);
        assert_token!("%", Operator);
        assert_token!("~", Operator);
        assert_token!("&", Operator);
        assert_token!("|", Operator);
        assert_token!("^", Operator);
        assert_tokens!(
            "1 + 2+3 -4-5 * 6*7 / 8/9",
            ["1", "+", "2", "+", "3", "-", "4", "-", "5", "*", "6", "*", "7", "/", "8", "/", "9"]
        );
        assert_tokens!("123::TEXT '2024-08-22'::DATE", ["123", "::", "TEXT", "'2024-08-22'", "::", "DATE"]);
    }

    #[test]
    fn test_parenthesis() {
        assert_tokens!("SELECT (1 + 2) * 3", ["SELECT", "(", "1", "+", "2", ")", "*", "3"]);
        // A missing opening parenthesis should not stop the tokenizer when reaching a closing parenthesis.
        assert_tokens!("SELECT 1 + 2) + 3; SELECT 2", ["SELECT", "1", "+", "2", ")", "+", "3", ";"], ["SELECT", "2"]);
        // A missing closing parenthesis should not prevent the tokenizer to stop when reaching the statement delimiter.
        assert_tokens!("SELECT (1 + 2 + 3; SELECT 2", ["SELECT", "(", "1", "+", "2", "+", "3", ";"], ["SELECT", "2"]);
    }

    #[test]
    fn test_delimited_token() {
        assert_token!("$$O'Reilly$$", QuotedIdentifierOrConstant);
        assert_token!("$tag$with_tag$tag$", QuotedIdentifierOrConstant);
        assert_token!("$x$__$__$x$", QuotedIdentifierOrConstant);
        assert_tokens!("$$O'Reilly", ["$$O'Reilly"]);
    }

    #[test]
    fn test_comment_token() {
        // multi-line comment
        assert_token!("/* / */", Comment);
        assert_token!("/** comment **/", Comment);
        assert_token!("/* comment */", Comment);
        assert_token!("/* /*nested*/comment */", Comment);
        assert_token!("/*+ SET_VAR(foreign_key_checks=OFF) */", Comment);
        assert_tokens!("BEGIN /* not closed...", ["BEGIN", "/* not closed..."]);
        assert_tokens!("BEGIN /* not closed...; BEGIN", ["BEGIN", "/* not closed...; BEGIN"]);
        assert_tokens!("/* line 1 \r\n line 2 */", ["/* line 1 \r\n line 2 */"]);

        // single-line comment
        assert_tokens!(
            "-- comment\n--comment\n# comment\n#comment",
            ["-- comment", "--comment", "# comment", "#comment"]
        );
    }

    #[test]
    fn test_quoted_identifier_or_constant() {
        assert_token!(r#"''"#, QuotedIdentifierOrConstant); // empty
        assert_token!(r#""""ID""""#, QuotedIdentifierOrConstant); // "ID"
        assert_token!(r#""""#, QuotedIdentifierOrConstant); // empty
        assert_token!(r#""ID ""X""""#, QuotedIdentifierOrConstant); // ID "X"
        assert_token!(r#"''''"#, QuotedIdentifierOrConstant); // A single quote, SELECT '''' -> '
        assert_token!(r#"'O''Reilly'"#, QuotedIdentifierOrConstant); // O'Reilly
        assert_tokens!("'missing ''end quote", ["'missing ''end quote"]);
        // string constant followed by a CAST identifier (PostgreSQL).
        assert_tokens!("'2024-08-22'::DATE", ["'2024-08-22'", "::", "DATE"]);
    }

    #[test]
    fn test_split_statements() {
        let s: Vec<_> = Tokenizer::new("SELECT 1; SELECT 2", Options::default()).collect();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].sql(), "SELECT 1;");
        assert_eq!(s[1].sql(), "SELECT 2");
    }

    #[test]
    fn test_empty_input() {
        let s: Vec<_> = Tokenizer::new("", Options::default()).collect();
        assert_eq!(s.len(), 0);
        let s: Vec<_> = Tokenizer::new(" \r\n ", Options::default()).collect();
        assert_eq!(s.len(), 0);
        let s: Vec<_> = Tokenizer::new("\r\n", Options::default()).collect();
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_some_random_syntax_edge_cases() {
        assert_token!(".", Any);
        assert_tokens!(".x2", [".", "x2"]);
    }
}
