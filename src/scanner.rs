use crate::tokens::{Token, TokenValue, Tokens};
use crate::SqlStatement;

pub(crate) struct Scanner<'s> {
    input: &'s str,

    /// The offset of the next character to be read from the input
    next_offset: usize,

    /// The current line of the scanner.
    line: usize,

    /// The current column of the scanner.
    column: usize,

    /// The offset of the start of the token currently being scanned.
    token_start_offset: usize,
}

impl<'s> Iterator for Scanner<'s> {
    type Item = SqlStatement<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_offset >= self.input.len() {
            return None;
        }
        // The start of the next statement is where the scanner is currently positioned.
        let next = &self.input[self.next_offset..];
        let mut input_iter = next.chars();
        self.get_statement(input_iter.by_ref())
    }
}

impl<'s> Scanner<'s> {
    pub(crate) fn new(input: &'s str) -> Self {
        Scanner { input, next_offset: 0, line: 1, column: 1, token_start_offset: 0 }
    }

    // The current offset of the scanner.
    // This is the offset of the last character read from the input.
    #[inline]
    fn offset(&self) -> usize {
        self.next_offset - 1
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
        if end_offset > self.token_start_offset {
            let value = value_constructor(&self.input[self.token_start_offset..end_offset]).into();
            tokens.tokens.push(Token::new(
                value,
                self.token_start_offset,
                end_offset,
                self.line,
                self.column,
                self.line,
                self.column,
            ));
        }
        self.token_start_offset = next_token_offset;
    }

    // Can be either `--` or `#`.
    // The `--` single-line comment is the most universally supported across different SQL dialects.
    // The `#`` single-line comment is less common and is primarily used in MySQL.
    fn capture_single_line_comment(&mut self, input_iter: &mut std::str::Chars, tokens: &mut Tokens<'s>) {
        while let Some(c) = self.get_next_char(input_iter) {
            if c == '\n' {
                self.line += 1;
                self.column = 1;
                break;
            }
        }
        self.capture_token(tokens, self.offset(), self.next_offset, TokenValue::Comment);
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
    // character to be processed by the scanner.
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
                    // We return the next character to the scanner so it can be processed.
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

    // Skip the parentheses.
    fn skip_parentheses(&mut self) {
        todo!()
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

    // Skip n characters from the iterator.
    // That function is expecting that the iterator contains at least n more characters and there are no new lines
    // skipped.
    #[inline]
    fn skip(&mut self, input_iter: &mut std::str::Chars, n: usize) {
        if n > 0 {
            input_iter.nth(n - 1);
            self.next_offset += n;
            self.column += n;
        }
    }

    fn get_statement(&mut self, input_iter: &mut std::str::Chars) -> Option<SqlStatement<'s>> {
        loop {
            // Capture the start of the statement.
            let start_line = self.line;
            let start_column = self.column;
            let start_offset = self.next_offset;

            // Reset the offset of the next token to the start of the statement.
            self.token_start_offset = self.next_offset;

            // A list of tokens that will be used to determine if the statement is a query or a command.
            // Tokens are captured at the top level so tokens found in CTEs or sub queries are not included in this list.
            // A token can be either a keyword, an identifier, a string literal, a number, a function name,
            // an expression such as '1+1', etc. Because everything between parentheses is skipped by the scanner, they
            // are not included in the list of tokens. Same applies to comments and string literals and quoted
            // identifiers.
            let mut tokens: Tokens = Tokens { tokens: Vec::new() };

            let mut next_char = self.get_next_char(input_iter);
            while let Some(c) = next_char {
                if c == '\n' {
                    //
                    // New Line.
                    //
                    self.capture_token(&mut tokens, self.offset(), self.next_offset, TokenValue::Any);
                    self.line += 1;
                    self.column = 1;
                } else if c == '\r' {
                    //
                    // Carriage Return (ignored).
                    //
                    self.capture_token(&mut tokens, self.offset(), self.next_offset, TokenValue::Any);
                    self.column -= 1;
                } else if c == ';' {
                    //
                    // Statement delimiter.
                    //
                    // We found the delimiter, it's the end of the statement.
                    self.capture_token(&mut tokens, self.offset(), self.next_offset, TokenValue::Any);
                    break;
                } else if c.is_whitespace() {
                    //
                    // Whitespace (could be \s, \t, \r, \n, etc.).
                    //
                    self.capture_token(&mut tokens, self.offset(), self.next_offset, TokenValue::Any);
                } else if c == '-' {
                    //
                    // Either a single-line comment '--' or a minus sign.
                    //
                    next_char = self.get_next_char(input_iter);
                    if next_char.as_ref() == Some(&'-') {
                        self.capture_token(&mut tokens, self.offset() - 1, self.offset() - 1, TokenValue::Comment);
                        self.capture_single_line_comment(input_iter, &mut tokens);
                    } else {
                        continue;
                    }
                } else if c == '#' {
                    //
                    // Single-line comment starting by '#' (MySQL).
                    //
                    self.capture_token(&mut tokens, self.offset(), self.offset(), TokenValue::Comment);
                    self.capture_single_line_comment(input_iter, &mut tokens);
                } else if c == '/' {
                    //
                    // Either a multi-line comment '/* ... */' or a division operator.
                    //
                    next_char = self.get_next_char(input_iter);
                    if next_char.as_ref() == Some(&'*') {
                        self.capture_token(&mut tokens, self.offset() - 1, self.offset() - 1, TokenValue::Comment);
                        self.capture_multi_line_comment(input_iter, &mut tokens);
                    } else {
                        continue;
                    }
                } else if c == '"' || c == '`' || c == '\'' {
                    //
                    // Quoted identifier or String literal.
                    //
                    next_char = self.capture_quoted_token(input_iter, c, &mut tokens);
                    continue;
                } else if c == '$' {
                    //
                    // May be dollar quoting (PostgreSQL).
                    //
                    // Before starting to identify the dollar-quoted delimiter we need to capture the current token.
                    self.capture_token(&mut tokens, self.offset(), self.offset(), TokenValue::Any);

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
                        let delimiter = &self.input[self.token_start_offset..self.next_offset];
                        next_char = self.capture_delimited_token(input_iter, delimiter, &mut tokens);
                    }
                    continue;
                } else if c == '(' {
                    //
                    // Start of a parentheses block.
                    //
                    self.skip_parentheses();
                } else if !c.is_alphanumeric() && c != '_' {
                    //
                    // Any other character that is not an underscore or alphanumeric will be considered as a boundary
                    // for a token.
                    //
                    self.capture_token(&mut tokens, self.offset(), self.next_offset, TokenValue::Any);
                }
                next_char = self.get_next_char(input_iter);
            }

            if next_char.is_none() {
                // We've reached the end of the input, we may have a last token to capture.
                self.capture_token(&mut tokens, self.next_offset, self.next_offset, TokenValue::Any);
            }

            // Before returning the statement we'll trim it to remove leading and trailing whitespaces & newlines.
            let sql = self.input[start_offset..self.next_offset]
                .trim_start_matches(char::is_whitespace)
                .trim_end_matches(|c: char| c.is_whitespace() || c == ';');

            // If there is no SQL statement (typically empty lines between two semicolons), we continue to scan
            // until we reach the end of the input or find a statement.
            if !sql.is_empty() {
                return Some(SqlStatement { sql, start_line, start_column, tokens });
            } else if next_char.is_none() {
                // We reached the end of the input.
                return None;
            }
        }
    }

    // Capture a token delimited by the given delimiter.
    //
    // The delimiter can be a single character or a multi-character delimiter.
    // There is no escaping mechanism for delimiters, so if the delimiter is found the token is captured and the next
    // character is returned to the scanner.
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
                    // We return the next character to the scanner so it can be processed.
                    self.skip(input_iter, delimiter.len() - 1);
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
}
