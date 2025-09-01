use anyhow::{bail, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum SqlQuery {
    SelectCount { table_name: String },
}

impl SqlQuery {
    pub fn parse(query: &str) -> Result<Self> {
        let query = query.trim();
        
        // Basic parsing for SELECT COUNT(*) FROM table_name
        if query.to_uppercase().starts_with("SELECT COUNT(*)") {
            let parts: Vec<&str> = query.split_whitespace().collect();
            
            if parts.len() != 4 {
                bail!("Invalid SELECT COUNT(*) syntax. Expected: SELECT COUNT(*) FROM table_name");
            }
            
            if parts[2].to_uppercase() != "FROM" {
                bail!("Expected FROM keyword in SELECT COUNT(*) query");
            }
            
            let table_name = parts[3].to_string();
            
            return Ok(SqlQuery::SelectCount { table_name });
        }
        
        bail!("Unsupported SQL query: {}", query);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_count_parsing() {
        let query = "SELECT COUNT(*) FROM apples";
        let parsed = SqlQuery::parse(query).unwrap();
        assert_eq!(parsed, SqlQuery::SelectCount { table_name: "apples".to_string() });
    }

    #[test]
    fn test_case_insensitive_parsing() {
        let query = "select count(*) from apples";
        let parsed = SqlQuery::parse(query).unwrap();
        assert_eq!(parsed, SqlQuery::SelectCount { table_name: "apples".to_string() });
    }

    #[test]
    fn test_invalid_query() {
        let query = "SELECT * FROM apples";
        assert!(SqlQuery::parse(query).is_err());
    }
}