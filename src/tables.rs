use std::collections::HashMap;
use uuid;

#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
}

impl TableSchema {
    pub fn new(name: impl Into<String>, columns: Vec<ColumnSchema>) -> Self {
        Self {
            name: name.into(),
            columns,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColumnSchema {
    pub id: ColumnId,
    pub name: String,
    pub ty: ColumnType,
}

impl ColumnSchema {
    pub fn new(name: impl Into<String>, ty: ColumnType) -> Self {
        Self {
            id: ColumnId::new(),
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ColumnType {
    String,
    Integer,
    Boolean,
    Real,
    DateTime,
    Reference { table: String },
    Select { options: Vec<SelectOption> },
}

#[derive(Debug, Clone)]
pub struct SelectOption {
    pub id: OptionId,
    pub label: String,
}

impl SelectOption {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: OptionId::new(),
            label: label.into(),
        }
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
        let reps = ColumnSchema::new("reps", ColumnType::Integer);
        let sets = ColumnSchema::new("sets", ColumnType::Integer);
        let exercise = ColumnSchema::new("exercise", ColumnType::String);

        assert_eq!(reps.name, "reps");
        assert!(matches!(reps.ty, ColumnType::Integer));
        assert_ne!(reps.id.0, sets.id.0);
        assert_ne!(reps.id.0, exercise.id.0);

        let table_schema = TableSchema::new(
            "workouts",
            vec![reps.clone(), sets.clone(), exercise.clone()],
        );

        assert_eq!(table_schema.name, "workouts");
        assert_eq!(table_schema.columns.len(), 3);
    }

    #[test]
    fn row_holds_a_value_per_column() {
        let reps = ColumnSchema::new("reps", ColumnType::Integer);
        let sets = ColumnSchema::new("sets", ColumnType::Integer);
        let exercise = ColumnSchema::new("exercise", ColumnType::String);

        let row = Row {
            id: RowId::new(),
            table: TableId::new(),
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
