use nom::{
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::{char, multispace0, multispace1},
    combinator::recognize,
    multi::separated_list1,
    sequence::tuple,
    IResult,
};

// Statement AST types
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    SelectStmt { 
        count_only: bool, 
        columns: Vec<String>,
        table_name: String 
    },
}

/// Main entry point for parsing SQL statements
pub fn parse_sql(query: &str) -> anyhow::Result<Statement> {
    match select_statement(query.trim()) {
        Ok((_, statement)) => Ok(statement),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            anyhow::bail!("Failed to parse SQL statement: {:?}", e.code);
        }
        Err(nom::Err::Incomplete(_)) => {
            anyhow::bail!("Incomplete SQL statement");
        }
    }
}

/// Parse SELECT statements
fn select_statement(input: &str) -> IResult<&str, Statement> {
    let (input, _) = tag_no_case("SELECT")(input)?;
    let (input, _) = multispace1(input)?;
    
    alt((
        select_count_statement,
        select_columns_statement,
    ))(input)
}

/// Parse SELECT COUNT(*) FROM table
fn select_count_statement(input: &str) -> IResult<&str, Statement> {
    let (input, _) = tuple((
        tag_no_case("COUNT"),
        multispace0,
        char('('),
        multispace0,
        char('*'),
        multispace0,
        char(')'),
        multispace1,
        tag_no_case("FROM"),
        multispace1,
    ))(input)?;
    
    let (input, table_name) = identifier(input)?;
    
    Ok((input, Statement::SelectStmt {
        count_only: true,
        columns: vec![],
        table_name: table_name.to_string(),
    }))
}

/// Parse SELECT column1, column2 FROM table
fn select_columns_statement(input: &str) -> IResult<&str, Statement> {
    let (input, columns) = separated_list1(
        tuple((multispace0, char(','), multispace0)),
        identifier
    )(input)?;
    
    let (input, _) = tuple((
        multispace1,
        tag_no_case("FROM"),
        multispace1,
    ))(input)?;
    
    let (input, table_name) = identifier(input)?;
    
    Ok((input, Statement::SelectStmt {
        count_only: false,
        columns: columns.into_iter().map(|s| s.to_string()).collect(),
        table_name: table_name.to_string(),
    }))
}

/// Parse SQL identifiers (table names, column names, etc.)
fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(tuple((
        nom::character::complete::satisfy(|c| c.is_ascii_alphabetic() || c == '_'),
        nom::bytes::complete::take_while(|c: char| c.is_ascii_alphanumeric() || c == '_')
    )))(input)
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
                columns: vec![],
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
                columns: vec![],
                table_name: "apples".to_string()
            }
        );
    }

    #[test]
    fn test_select_single_column() {
        let query = "SELECT name FROM apples";
        let parsed = parse_sql(query).unwrap();
        assert_eq!(
            parsed,
            Statement::SelectStmt {
                count_only: false,
                columns: vec!["name".to_string()],
                table_name: "apples".to_string()
            }
        );
    }

    #[test]
    fn test_select_multiple_columns() {
        let query = "SELECT name, color FROM apples";
        let parsed = parse_sql(query).unwrap();
        assert_eq!(
            parsed,
            Statement::SelectStmt {
                count_only: false,
                columns: vec!["name".to_string(), "color".to_string()],
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
