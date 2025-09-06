use nom::{
    branch::alt,
    bytes::complete::{tag_no_case, take_while1, take_while},
    character::complete::{char, multispace0, multispace1},
    combinator::{opt, recognize},
    multi::separated_list1,
    sequence::{delimited, tuple},
    IResult,
};

/// SQL data types supported by SQLite
#[derive(Debug, Clone, PartialEq)]
pub enum SqlType {
    Integer,
    Text,
    Real,
    Blob,
    Numeric,
}

impl SqlType {
    pub fn from_str(type_str: &str) -> anyhow::Result<Self> {
        match type_str.to_uppercase().as_str() {
            "INTEGER" => Ok(Self::Integer),
            "TEXT" => Ok(Self::Text),
            "REAL" => Ok(Self::Real),
            "BLOB" => Ok(Self::Blob),
            "NUMERIC" => Ok(Self::Numeric),
            other => anyhow::bail!("Unsupported SQL type: {}", other),
        }
    }
}

/// Represents a column definition in a CREATE TABLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDefinition {
    pub name: String,
    pub sql_type: SqlType,
    pub position: usize,
    pub is_primary_key: bool,
}

/// Intermediate parsing result for column specifications
#[derive(Debug, Clone)]
struct ColumnSpecDto {
    name: String,
    sql_type: SqlType,
    is_primary_key: bool,
}

/// Represents a parsed table schema from CREATE TABLE statement  
#[derive(Debug, Clone, PartialEq)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub rootpage: i64,
}

impl TableSchema {
    /// Resolve column names to their indices in the table
    pub fn resolve_column_indices(&self, column_names: &[String]) -> anyhow::Result<Vec<usize>> {
        let mut indices = Vec::new();
        
        for column_name in column_names {
            if let Some(column) = self.columns.iter().find(|col| col.name == *column_name) {
                indices.push(column.position);
            } else {
                anyhow::bail!("Column '{}' not found in table '{}'", column_name, self.name);
            }
        }
        
        Ok(indices)
    }

    /// Resolve column names to their definitions in the table
    pub fn resolve_columns(&self, column_names: &[String]) -> anyhow::Result<Vec<&ColumnDefinition>> {
        let mut columns = Vec::new();
        
        for column_name in column_names {
            if let Some(column) = self.columns.iter().find(|col| col.name == *column_name) {
                columns.push(column);
            } else {
                anyhow::bail!("Column '{}' not found in table '{}'", column_name, self.name);
            }
        }
        
        Ok(columns)
    }
}

/// Parser for CREATE TABLE statements using nom combinators
pub struct TableSchemaParser;

impl TableSchemaParser {
    /// Parse CREATE TABLE SQL statement into structured schema
    pub fn parse_create_table_sql(create_sql: &str) -> anyhow::Result<(String, Vec<ColumnDefinition>)> {
        match Self::create_table_statement(create_sql.trim()) {
            Ok((_, (table_name, columns))) => Ok((table_name, columns)),
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                anyhow::bail!("Failed to parse CREATE TABLE statement: {:?}", e.code);
            }
            Err(nom::Err::Incomplete(_)) => {
                anyhow::bail!("Incomplete CREATE TABLE statement");
            }
        }
    }

    /// Main parser for CREATE TABLE statement
    fn create_table_statement(input: &str) -> IResult<&str, (String, Vec<ColumnDefinition>)> {
        let (input, _) = tuple((
            tag_no_case("CREATE"),
            multispace1,
            tag_no_case("TABLE"),
            multispace1,
        ))(input)?;

        let (input, table_name) = Self::identifier(input)?;
        let (input, _) = multispace0(input)?;
        
        let (input, columns) = delimited(
            char('('),
            Self::column_definitions,
            char(')')
        )(input)?;

        Ok((input, (table_name.to_string(), columns)))
    }

    /// Parse column definitions inside parentheses
    fn column_definitions(input: &str) -> IResult<&str, Vec<ColumnDefinition>> {
        let (input, _) = multispace0(input)?;
        
        let (input, column_specs) = separated_list1(
            tuple((multispace0, char(','), multispace0)),
            Self::column_definition
        )(input)?;
        
        let (input, _) = multispace0(input)?;
        
        // Assign positions to columns
        let columns = column_specs
            .into_iter()
            .enumerate()
            .map(|(position, spec)| ColumnDefinition {
                name: spec.name,
                sql_type: spec.sql_type,
                position,
                is_primary_key: spec.is_primary_key,
            })
            .collect();
            
        Ok((input, columns))
    }

    /// Parse a single column definition
    fn column_definition(input: &str) -> IResult<&str, ColumnSpecDto> {
        let (input, _) = multispace0(input)?;
        let (input, column_name) = Self::identifier(input)?;
        let (input, _) = multispace1(input)?;
        let (input, sql_type) = Self::sql_type(input)?;
        let (input, _) = multispace0(input)?;
        
        // Check for PRIMARY KEY constraint
        let (input, is_primary_key) = opt(tuple((
            tag_no_case("PRIMARY"),
            multispace1,
            tag_no_case("KEY"),
        )))(input)?;
        
        // Skip any additional constraints for now (like AUTOINCREMENT)
        let (input, _) = take_while(|c: char| c != ',' && c != ')')(input)?;
        
        let parsed_sql_type = SqlType::from_str(sql_type)
            .map_err(|_| nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))?;

        Ok((input, ColumnSpecDto {
            name: column_name.to_string(),
            sql_type: parsed_sql_type,
            is_primary_key: is_primary_key.is_some(),
        }))
    }

    /// Parse SQL data types
    fn sql_type(input: &str) -> IResult<&str, &str> {
        alt((
            tag_no_case("INTEGER"),
            tag_no_case("TEXT"),
            tag_no_case("REAL"),
            tag_no_case("BLOB"),
            tag_no_case("NUMERIC"),
        ))(input)
    }

    /// Parse SQL identifiers (table names, column names, etc.)
    fn identifier(input: &str) -> IResult<&str, &str> {
        recognize(tuple((
            alt((
                take_while1(|c: char| c.is_ascii_alphabetic() || c == '_'),
                take_while1(|c: char| c.is_ascii_digit())
            )),
            take_while(|c: char| c.is_ascii_alphanumeric() || c == '_')
        )))(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_create_table() {
        let sql = "CREATE TABLE apples (id INTEGER PRIMARY KEY, name TEXT)";
        let result = TableSchemaParser::parse_create_table_sql(sql).unwrap();
        
        assert_eq!(result.0, "apples");
        assert_eq!(result.1.len(), 2);
        
        let id_col = &result.1[0];
        assert_eq!(id_col.name, "id");
        assert_eq!(id_col.sql_type, SqlType::Integer);
        assert_eq!(id_col.position, 0);
        assert!(id_col.is_primary_key);
        
        let name_col = &result.1[1];
        assert_eq!(name_col.name, "name");
        assert_eq!(name_col.sql_type, SqlType::Text);
        assert_eq!(name_col.position, 1);
        assert!(!name_col.is_primary_key);
    }

    #[test]
    fn test_create_table_with_autoincrement() {
        let sql = "CREATE TABLE apples (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, color TEXT)";
        let result = TableSchemaParser::parse_create_table_sql(sql).unwrap();
        
        assert_eq!(result.0, "apples");
        assert_eq!(result.1.len(), 3);
        
        // Find columns by name
        let id_col = result.1.iter().find(|col| col.name == "id").unwrap();
        let name_col = result.1.iter().find(|col| col.name == "name").unwrap();
        let color_col = result.1.iter().find(|col| col.name == "color").unwrap();
            
        assert!(id_col.is_primary_key);
        assert_eq!(name_col.position, 1);
        assert_eq!(color_col.position, 2);
    }

    #[test]
    fn test_multiline_create_table() {
        let sql = r#"CREATE TABLE apples
        (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT,
            color TEXT
        )"#;
        
        let result = TableSchemaParser::parse_create_table_sql(sql).unwrap();
        assert_eq!(result.0, "apples");
        assert_eq!(result.1.len(), 3);
    }
}