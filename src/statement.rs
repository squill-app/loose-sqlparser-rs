use crate::{tokens::Tokens, Position, TokenValue};

#[cfg(feature = "serialize")]
use serde::Serialize;

// A SQL statement.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(Serialize))]
pub struct Statement<'s> {
    // The input from which the statement was parsed.
    pub(crate) input: &'s str,

    // All tokens found in the statement.
    pub(crate) tokens: Tokens<'s>,
}

impl Statement<'_> {
    /// The SQL statement.
    pub fn sql(&self) -> &str {
        return &self.input[self.start().offset..self.end().offset];
    }

    /// The start position of the statement.
    pub fn start(&self) -> &Position {
        &self.tokens[0].start
    }

    /// The column where the statement starts.
    pub fn end(&self) -> &Position {
        &self.tokens[self.tokens.len() - 1].end
    }

    pub fn tokens(&self) -> &Tokens<'_> {
        &self.tokens
    }

    /// The list of keywords found in the statement at the top level.
    /// Keywords found on CTEs or sub queries are not included in this list.
    pub fn keywords(&self) -> Vec<&str> {
        self.tokens
            .iter()
            .filter(|token| match token.value {
                TokenValue::IdentifierOrKeyword(value) => value.chars().all(|c| c.is_ascii_alphabetic()),
                _ => false,
            })
            .map(|token| token.value.as_ref())
            .collect()
    }

    /// Returns whether the statement is empty.
    ///
    /// An empty statement is a statement that contains nothing else that comments or whitespace.
    pub fn is_empty(&self) -> bool {
        self.tokens.iter().all(|t| t.is_comment() || t.is_statement_delimiter())
    }

    /// Returns whether the statement may return rows.
    ///
    /// Used to categorize statements that return rows versus those that don't return rows.
    /// The following SQL statements are considered queries:
    /// - SELECT ... (excluding SELECT INTO)
    /// - SHOW ...
    /// - DESCRIBE ...
    /// - EXPLAIN ...
    /// - WITH ... SELECT ...
    /// - VALUES ...
    /// - LIST ...
    /// - PRAGMA ...
    /// - INSERT|UPDATE|DELETE ... RETURNING ...
    ///
    /// This function is a heuristic and may not be accurate in all cases.
    pub fn is_query(&self) -> bool {
        let keywords = self.keywords();
        if keywords.is_empty() {
            return false;
        }
        // 1. The statement starts with a keyword that is unambiguously a query.
        (matches!(keywords[0].to_uppercase().as_str(),
            "SHOW" | "DESCRIBE" | "EXPLAIN" | "VALUES" | "LIST" | "PRAGMA"))
        // 2. The statement starts with a WITH clause followed by a SELECT or RETURNING.
            || (keywords[0].to_uppercase() == "WITH"
                && keywords.iter().any(|&k| matches!(k.to_uppercase().as_str(), "SELECT" | "RETURNING")))
        // 3. The statement is an INSERT, UPDATE, or DELETE with a RETURNING clause.
            || (matches!(keywords[0].to_uppercase().as_str(), "INSERT" | "UPDATE" | "DELETE")
                && keywords.iter().any(|&k| k.to_uppercase().as_str() == "RETURNING"))
        // 4. The statement is a SELECT (except SELECT ... INTO).
            || (keywords[0].to_uppercase() == "SELECT"
                && !keywords.iter().any(|&k| k.to_uppercase().as_str() == "INTO"))
    }
}

#[cfg(test)]
mod tests {
    use crate::loose_sqlparse;

    #[test]
    fn test_statement_is_empty() {
        let statements: Vec<_> = loose_sqlparse("SELECT 1;\n\t \n;;SELECT 2").collect();
        assert_eq!(statements.len(), 4);
        assert!(!statements[0].is_empty());
        assert!(statements[1].is_empty());
        assert!(statements[2].is_empty());
        assert!(!statements[3].is_empty());
    }
}
