use std::rc::Rc;
use anyhow::bail;

// Token types for SQL lexical analysis
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Select,
    Count,
    LParen,
    Star,
    RParen,
    From,
    Identifier(String),
}

// Statement AST types
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    SelectStmt { count_only: bool, table_name: String },
}

// Tokeniser
fn tokenise(input: &str) -> anyhow::Result<Rc<[Token]>> {
    let mut tokens = Vec::new();
    let mut chars = input.trim().chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
                continue;
            }
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            '*' => {
                tokens.push(Token::Star);
                chars.next();
            }
            c if c.is_ascii_alphabetic() => {
                let mut word = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        word.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                match word.to_uppercase().as_str() {
                    "SELECT" => tokens.push(Token::Select),
                    "COUNT" => tokens.push(Token::Count),
                    "FROM" => tokens.push(Token::From),
                    _ => tokens.push(Token::Identifier(word)),
                }
            }
            _ => bail!("Unexpected character: '{}'", ch),
        }
    }

    Ok(tokens.into())
}

// Parser
pub fn parse_sql(query: &str) -> anyhow::Result<Statement> {
    let tokens = tokenise(query)?;
    parse_tokens(&tokens)
}

fn parse_tokens(tokens: &[Token]) -> anyhow::Result<Statement> {
    if tokens.is_empty() {
        bail!("Empty query");
    }

    match tokens[0] {
        Token::Select => parse_select_statement(tokens),
        _ => bail!("Only SELECT statements are supported"),
    }
}

fn parse_select_statement(tokens: &[Token]) -> anyhow::Result<Statement> {
    let expected_pattern = [
        Token::Select,
        Token::Count,
        Token::LParen,
        Token::Star,
        Token::RParen,
        Token::From,
    ];

    if tokens.len() < expected_pattern.len() + 1 {
        bail!("Invalid SELECT COUNT(*) syntax - missing tokens");
    }

    for (idx, expected_token) in expected_pattern.iter().enumerate() {
        if tokens[idx] != *expected_token {
            bail!(
                "Invalid SELECT COUNT(*) syntax at position {}: expected {:?}, got {:?}",
                idx,
                expected_token,
                tokens[idx]
            );
        }
    }

    if let Token::Identifier(table_name) = &tokens[expected_pattern.len()] {
        if tokens.len() != expected_pattern.len() + 1 {
            bail!("Extra tokens after table name");
        }

        Ok(Statement::SelectStmt {
            count_only: true,
            table_name: table_name.clone(),
        })
    } else {
        bail!(
            "Expected table name after FROM, got {:?}",
            tokens[expected_pattern.len()]
        );
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_count_parsing() {
        let query = "SELECT COUNT(*) FROM apples";
        let parsed = parse_sql(query).unwrap();
        assert_eq!(
            parsed,
            Statement::SelectStmt {
                count_only: true,
                table_name: "apples".to_string()
            }
        );
    }

    #[test]
    fn test_case_insensitive_parsing() {
        let query = "select count(*) from apples";
        let parsed = parse_sql(query).unwrap();
        assert_eq!(
            parsed,
            Statement::SelectStmt {
                count_only: true,
                table_name: "apples".to_string()
            }
        );
    }

    #[test]
    fn test_invalid_query() {
        let query = "SELECT * FROM apples";
        assert!(parse_sql(query).is_err());
    }
}
