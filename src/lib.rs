use scanner::Scanner;
use statement::SqlStatement;

mod scanner;
pub mod statement;
pub mod tokens;

/// A simple loose SQL parser that can split SQL statements from a script.
///
/// This scanner is not validating.
///
/// # Not Supported
/// - Square Brackets ([...]) for delimiters (T-SQL / SQL Server Specific)
///   ```sql
///     SELECT [Column1], [Column2] FROM [Table]
///     ```
/// -  `q` notation for string literals (Oracle Specific)
///   ```sql
///     SELECT q'{It's a sunny day}';
///     ```
/// - Use a backslash (\) to escape a single quote (MySQL non-strict mode).
///   ```sql
///     SELECT 'O\'Reilly';
///     ```

/// Scans a SQL string and returns an iterator over the statements.
///
/// This is a non-validating SQL scanner, it will not check the syntax validity of the SQL statements,
/// it will only separate them by the delimiter.
///
/// The iterator will return a {{SqlStatement}} for each statement found in the input string.
/// Statements are separated by a delimiter (default is semicolon `;`).
pub fn loose_sqlparse(sql: &str) -> impl Iterator<Item = SqlStatement<'_>> {
    Scanner::new(sql)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_delimited_token() {
        let s: Vec<_> = loose_sqlparse("SELECT $$O'Reilly$$, $tag$with_tag$tag$, $x$__$__$x$ FROM t1").collect();
        assert_eq!(s[0].tokens(), &["SELECT", "$$O'Reilly$$", "$tag$with_tag$tag$", "$x$__$__$x$", "FROM", "t1"]);
    }

    #[test]
    fn test_capture_quoted_token() {
        let statements: Vec<_> = loose_sqlparse("SELECT '', '''' FROM t1").collect();
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].sql, "SELECT '', '''' FROM t1");
        assert_eq!(statements[0].tokens(), &["SELECT", "''", "''''", "FROM", "t1"]);

        let statements: Vec<_> = loose_sqlparse("SELECT 'O''Reilly' FROM t1; SELECT 'O''Reilly FROM t2").collect();
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].sql, "SELECT 'O''Reilly' FROM t1");
        assert_eq!(statements[0].tokens(), &["SELECT", "'O''Reilly'", "FROM", "t1"]);
        assert_eq!(statements[1].sql, "SELECT 'O''Reilly FROM t2");
        assert_eq!(statements[1].tokens(), &["SELECT", "'O''Reilly FROM t2"]);

        let statements: Vec<_> = loose_sqlparse(r#"SELECT 1000 AS "ID ""X""" FROM test; SELECT 2"#).collect();
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].sql, r#"SELECT 1000 AS "ID ""X""" FROM test"#);
        assert_eq!(statements[0].tokens(), &["SELECT", "1000", "AS", r#""ID ""X""""#, "FROM", "test"]);
        assert_eq!(statements[1].sql, "SELECT 2");

        // Should reach the end of the input without finding the end of the identifier
        let statements: Vec<_> = loose_sqlparse(r#"SELECT 1000 AS "ID ""X; SELECT 2"#).collect();
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].sql, r#"SELECT 1000 AS "ID ""X; SELECT 2"#);
        assert_eq!(statements[0].tokens(), &["SELECT", "1000", "AS", r#""ID ""X; SELECT 2"#]);
    }

    #[test]
    fn test_basics() {
        let sql = "SELECT 1; SELECT 2";
        let statements: Vec<_> = loose_sqlparse(sql).collect();
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].sql, "SELECT 1");
        assert_eq!(statements[1].sql, "SELECT 2");
    }

    #[test]
    fn test_single_line_comments() {
        let statements: Vec<_> = loose_sqlparse("SELECT 2-1; -- This is a comment\nSELECT 2").collect();
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].sql, "SELECT 2-1");
        assert_eq!(statements[0].tokens(), &["SELECT", "2-1"]);
        assert_eq!(statements[1].sql, "-- This is a comment\nSELECT 2");

        let statements: Vec<_> = loose_sqlparse("SELECT 1; # This is a comment\nSELECT 2").collect();
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].sql, "SELECT 1");
        assert_eq!(statements[0].tokens(), &["SELECT", "1"]);
        assert_eq!(statements[1].sql, "# This is a comment\nSELECT 2");
    }

    #[test]
    fn test_multi_line_comments() {
        let statements: Vec<_> =
            loose_sqlparse("SELECT /* comment */ 2/1;SELECT /** line1\n * line 2\n **/2").collect();
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].sql, "SELECT /* comment */ 2/1");
        assert_eq!(statements[0].tokens(), &["SELECT", "2/1"]);
        assert_eq!(statements[1].sql, "SELECT /** line1\n * line 2\n **/2");
        assert_eq!(statements[1].tokens(), &["SELECT", "2"]);
    }

    #[test]
    fn test_skip_empty_statements() {
        let statements: Vec<_> = loose_sqlparse("SELECT 1;\n\t \n; SELECT 2").collect();
        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].sql, "SELECT 1");
        assert_eq!(statements[1].sql, "SELECT 2");
        let statements: Vec<_> = loose_sqlparse("\n\t \n; SELECT 1;").collect();
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].sql, "SELECT 1");
        let statements: Vec<_> = loose_sqlparse("; SELECT 1").collect();
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].sql, "SELECT 1");
    }

    /*
            let s: Vec<_> = loose_sqlparse(r#"
            CREATE FUNCTION dup(in int, out f1 int, out f2 text)
                AS $$ SELECT $1, CAST($1 AS text) || ' is text' $$
                LANGUAGE SQL; SELECT * FROM dup(42);"#).collect();
            assert_eq!(s[0].tokens(), &["SELECT", "$$O'Reilly$$", "$tag$with_tag$tag$", "$x$__$__$x$", "FROM", "t1"]);
    */
}
