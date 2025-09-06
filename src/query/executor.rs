use std::fs::File;
use anyhow::{anyhow, bail, Result};

use crate::{
    Sqlite, 
    sql::Statement, 
    storage::{BTreePage, LeafTableCell}, 
    pager::PageNumber,
    schema::{TableSchema, TableSchemaParser, ColumnDefinition}
};

/// Query execution results
#[derive(Debug, Clone, PartialEq)]
pub struct QueryRow {
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    pub rows: Vec<QueryRow>,
}

impl QueryResult {
    pub fn empty() -> Self {
        Self { rows: Vec::new() }
    }
    
    pub fn single_value(value: String) -> Self {
        Self {
            rows: vec![QueryRow { values: vec![value] }],
        }
    }
    
    pub fn count(count: u16) -> Self {
        Self::single_value(count.to_string())
    }
}

/// Query executor using parameterised lifetime pattern for maximum flexibility.
///
/// Uses method-scoped lifetimes rather than struct-bound lifetimes to provide
/// flexible API design where each method call gets independent lifetime scope.
pub struct QueryExecutor;

impl QueryExecutor {
    /// Execute any SQL statement against the database.
    ///
    /// Currently, supports:
    /// - COUNT(*) queries 
    /// - Column selection (SELECT col1, col2)
    ///
    /// Future query execution plans could include:
    /// - Row filtering (WHERE conditions)
    /// - Table joins  
    /// - Aggregation operations (GROUP BY)
    /// - Sorting operations (ORDER BY)
    pub fn execute<'a>(
        &self,
        sqlite: &'a mut Sqlite<File>,
        statement: Statement,
    ) -> Result<QueryResult> {
        match statement {
            Statement::SelectStmt {
                count_only: true,
                table_name,
                ..
            } => self.execute_count(sqlite, &table_name),
            Statement::SelectStmt {
                count_only: false,
                columns,
                table_name,
            } => self.execute_select_columns(sqlite, &table_name, &columns),
        }
    }
    
    /// Execute COUNT(*) queries
    fn execute_count<'a>(
        &self,
        sqlite: &'a mut Sqlite<File>,
        table_name: &str,
    ) -> Result<QueryResult> {
        let schema_record = sqlite
            .schema_page
            .find_table(table_name)?
            .ok_or_else(|| anyhow!("Table '{}' not found", table_name))?;
        
        let table_page = sqlite.load_page(schema_record.rootpage as u64)?;
        
        if !table_page.is_table_page() {
            bail!(
                "Expected table page for table '{}' but got a different page type: {:?}",
                schema_record.name,
                table_page
            );
        }
        
        Ok(QueryResult::count(table_page.cell_count()))
    }
    
    /// Execute SELECT column queries  
    fn execute_select_columns<'a>(
        &self,
        sqlite: &'a mut Sqlite<File>,
        table_name: &str,
        columns: &[String],
    ) -> Result<QueryResult> {
        // Phase 1: Schema Resolution
        let schema_record = sqlite
            .schema_page
            .find_table(table_name)?
            .ok_or_else(|| anyhow!("Table '{}' not found", table_name))?;
        
        // Phase 2: Parse CREATE TABLE statement to understand column structure
        let (parsed_table_name, column_definitions) = 
            TableSchemaParser::parse_create_table_sql(&schema_record.sql)?;
        
        if parsed_table_name != table_name {
            bail!("Schema inconsistency: expected '{}', got '{}'", table_name, parsed_table_name);
        }
        
        let table_schema = TableSchema {
            name: parsed_table_name,
            columns: column_definitions,
            rootpage: schema_record.rootpage,
        };
        
        // Phase 3: Resolve column definitions
        let column_definitions = table_schema.resolve_columns(columns)?;
        
        // Phase 4: Load Table Page Buffer (we need the raw buffer for cell parsing)
        let page_num = PageNumber::new(schema_record.rootpage as u64)
            .map_err(|e| anyhow!("Invalid page number {}: {}", schema_record.rootpage, e))?;
        
        let mut page_buffer = vec![0u8; sqlite.header.page_size as usize];
        sqlite.pager.read(page_num, &mut page_buffer)?;
        
        // Phase 5: Parse page to get cell pointers
        let table_page = BTreePage::parse(&page_buffer)?;
        
        // Phase 6: Execute projection plan
        self.execute_projection(&table_page, &page_buffer, &column_definitions)
    }
    
    /// Execute projection operation - select specific columns from table rows
    fn execute_projection(
        &self,
        table_page: &BTreePage,
        page_buffer: &[u8],
        column_definitions: &[&ColumnDefinition],
    ) -> Result<QueryResult> {
        match table_page {
            BTreePage::LeafTable(leaf_page) => {
                let rows = leaf_page.cell_pointers
                    .iter()
                    .map(|&cell_offset| {
                        let cell = LeafTableCell::parse(page_buffer, cell_offset)?;
                        
                        let values = column_definitions
                            .iter()
                            .map(|&column_def| {
                                if column_def.position < cell.record_header.column_count() {
                                    cell.text_column(page_buffer, column_def)
                                } else {
                                    bail!("Column '{}' position {} out of bounds for table (has {} columns)", 
                                          column_def.name, column_def.position, cell.record_header.column_count())
                                }
                            })
                            .collect::<Result<Vec<_>>>()?;
                        
                        Ok(QueryRow { values })
                    })
                    .collect::<Result<Vec<_>>>()?;
                
                Ok(QueryResult { rows })
            }
            _ => bail!("Expected leaf table page for table data extraction"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_query_result_creation() {
        let empty = QueryResult::empty();
        assert_eq!(empty.rows.len(), 0);
        
        let single = QueryResult::single_value("test".to_string());
        assert_eq!(single.rows.len(), 1);
        assert_eq!(single.rows[0].values, vec!["test".to_string()]);
        
        let count = QueryResult::count(5);
        assert_eq!(count.rows.len(), 1);
        assert_eq!(count.rows[0].values, vec!["5".to_string()]);
    }
}