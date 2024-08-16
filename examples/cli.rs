use loose_sqlparser::{loose_sqlparse, tokens::Tokens};
use terminal_size::{terminal_size, Width};

const ELLIPSIS: &str = "...";
const METADATA_COL_WIDTH: usize = 33;

fn main() {
    let filename = std::env::args().nth(1).expect(r#"Usage: cargo run --example cli FILENAME.sql"#);
    let col_width: usize = terminal_size().map(|(Width(w), _)| w as usize).unwrap_or(80) - METADATA_COL_WIDTH;
    let sql = std::fs::read_to_string(filename).expect("Failed to read file");
    for statement in loose_sqlparse(&sql) {
        println!();
        println!("> {}", statement.sql());
        println!("  query: {}", if statement.is_query() { "yes" } else { "no" });
        println!("  empty: {}", if statement.is_empty() { "yes" } else { "no" });
        println!();
        println!("-------------------------------|-{}", "-".repeat(col_width));
        println!("     START     |      END      |");
        println!("-------------------------------| TOKEN");
        println!(" line  |  col  | line  |  col  |");
        println!("-------------------------------|-{}", "-".repeat(col_width));
        display_tokens(statement.tokens(), col_width, 0);
    }
}

fn display_tokens(tokens: &Tokens, col_width: usize, indent: usize) {
    for token in &tokens.tokens {
        if token.is_fragment() {
            display_tokens(token.children().unwrap(), col_width, indent + 2);
            continue;
        }

        let mut sql = token.value.as_ref().to_string().replace('\n', " ");
        if indent + ELLIPSIS.len() > col_width {
            sql = ELLIPSIS.to_string();
        } else if sql.len() > col_width - indent {
            sql.truncate(col_width - ELLIPSIS.len() - indent);
            sql.push_str(ELLIPSIS);
        }
        println!(
            " {:>5} | {:>5} | {:>5} | {:>5} | {:indent$}{}",
            token.start.line, token.start.column, token.end.line, token.end.column, "", sql
        );
    }
}
