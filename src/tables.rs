use std::collections::HashMap;
use uuid;

#[derive(Clone)]
pub struct TableSchema {
    pub id: TableId,
    pub name: String,
    pub columns: Vec<ColumnSchema>,
}

impl std::fmt::Debug for TableSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Table: id={} name={}", self.id.0, self.name)?;
        for (i, column) in self.columns.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "\t{column:?}")?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct ColumnSchema {
    pub id: ColumnId,
    pub table_id: TableId,
    pub name: String,
    pub ty: ColumnType,
}

impl std::fmt::Debug for ColumnSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Column: id={} table_id={} name={} type={}",
            self.id.0, self.table_id.0, self.name, self.ty
        )
    }
}

#[derive(Debug, Clone)]
pub enum ColumnType {
    String,
    Integer,
    Boolean,
    Real,
    DateTime,
}

impl std::fmt::Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ColumnType::String => "String",
            ColumnType::Integer => "Integer",
            ColumnType::Boolean => "Boolean",
            ColumnType::Real => "Real",
            ColumnType::DateTime => "Datetime",
        };

        f.write_str(s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    String(String),
    Integer(i64),
    Real(f64),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableId(pub uuid::Uuid);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColumnId(pub uuid::Uuid);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RowId(pub uuid::Uuid);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OptionId(uuid::Uuid);

impl TableId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl ColumnId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl RowId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl OptionId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

pub struct Row {
    pub id: RowId,
    pub table: TableId,
    pub values: HashMap<ColumnId, Value>,
}

impl Row {
    pub fn new(table: TableId, values: HashMap<ColumnId, Value>) -> Self {
        Self {
            id: RowId::new(),
            table,
            values,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_schema_new_sets_fields_and_generates_unique_column_ids() {
        let table_id = TableId::new();
        let reps = ColumnSchema {
            id: ColumnId::new(),
            table_id: table_id.clone(),
            name: "reps".into(),
            ty: ColumnType::Integer,
        };
        let sets = ColumnSchema {
            id: ColumnId::new(),
            table_id: table_id.clone(),
            name: "sets".into(),
            ty: ColumnType::Integer,
        };
        let exercise = ColumnSchema {
            id: ColumnId::new(),
            table_id: table_id.clone(),
            name: "exercise".into(),
            ty: ColumnType::String,
        };

        assert_eq!(reps.name, "reps");
        assert!(matches!(reps.ty, ColumnType::Integer));
        assert_eq!(reps.table_id, table_id);
        assert_ne!(reps.id.0, sets.id.0);
        assert_ne!(reps.id.0, exercise.id.0);

        let table_schema = TableSchema {
            id: table_id.clone(),
            name: "workouts".into(),
            columns: vec![reps.clone(), sets.clone(), exercise.clone()],
        };

        assert_eq!(table_schema.id, table_id);
        assert_eq!(table_schema.name, "workouts");
        assert_eq!(table_schema.columns.len(), 3);
    }

    #[test]
    fn row_holds_a_value_per_column() {
        let table_id = TableId::new();
        let reps = ColumnSchema {
            id: ColumnId::new(),
            table_id: table_id.clone(),
            name: "reps".into(),
            ty: ColumnType::Integer,
        };
        let sets = ColumnSchema {
            id: ColumnId::new(),
            table_id: table_id.clone(),
            name: "sets".into(),
            ty: ColumnType::Integer,
        };
        let exercise = ColumnSchema {
            id: ColumnId::new(),
            table_id: table_id.clone(),
            name: "exercise".into(),
            ty: ColumnType::String,
        };

        let row = Row {
            id: RowId::new(),
            table: table_id,
            values: HashMap::from([
                (reps.id.clone(), Value::Integer(10)),
                (sets.id.clone(), Value::Integer(3)),
                (exercise.id.clone(), Value::String("Squat".into())),
            ]),
        };

        assert_eq!(row.values.get(&reps.id), Some(&Value::Integer(10)));
        assert_eq!(row.values.get(&sets.id), Some(&Value::Integer(3)));
        assert_eq!(
            row.values.get(&exercise.id),
            Some(&Value::String("Squat".into()))
        );
    }
}
