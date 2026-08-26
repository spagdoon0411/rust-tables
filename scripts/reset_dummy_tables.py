#!/usr/bin/env python3
"""Clears table_schemas (and its backing physical tables/columns) in the
fitness-tracker SQLite database, then repopulates it with dummy tables.

Mirrors the schema src/repository.rs creates: each table_schemas row gets a
default "Name" TEXT column and a physical table named "<name>-<id>".
"""

import sqlite3
import uuid
from pathlib import Path

DB_PATH = Path(__file__).resolve().parent.parent / "user_data" / "fitness_tracker.db"

DUMMY_TABLE_NAMES = [f"Table{i}" for i in range(1, 21)]


def quote_ident(ident: str) -> str:
    return '"' + ident.replace('"', '""') + '"'


def physical_table_name(name: str, table_id: str) -> str:
    return f"{name}-{table_id}"


def clear_tables(conn: sqlite3.Connection) -> None:
    rows = conn.execute("SELECT id, name FROM table_schemas").fetchall()
    for table_id, name in rows:
        physical_name = physical_table_name(name, table_id)
        conn.execute(f"DROP TABLE IF EXISTS {quote_ident(physical_name)}")

    conn.execute("DELETE FROM column_schemas")
    conn.execute("DELETE FROM table_schemas")


def populate_dummy_tables(conn: sqlite3.Connection, names: list[str]) -> None:
    for name in names:
        table_id = str(uuid.uuid4())
        column_id = str(uuid.uuid4())

        conn.execute(
            "INSERT INTO table_schemas (id, name) VALUES (?, ?)",
            (table_id, name),
        )
        conn.execute(
            "INSERT INTO column_schemas (id, table_id, name, ty) VALUES (?, ?, ?, ?)",
            (column_id, table_id, "Name", "String"),
        )

        physical_name = physical_table_name(name, table_id)
        conn.execute(
            f"CREATE TABLE {quote_ident(physical_name)} "
            f"(id TEXT PRIMARY KEY, {quote_ident('Name')} TEXT)"
        )


def main() -> None:
    if not DB_PATH.is_file():
        raise SystemExit(f"no database found at {DB_PATH}")

    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA foreign_keys = ON")

    try:
        with conn:
            clear_tables(conn)
            populate_dummy_tables(conn, DUMMY_TABLE_NAMES)
    finally:
        conn.close()

    print(f"Reset {DB_PATH} with dummy tables: {', '.join(DUMMY_TABLE_NAMES)}")


if __name__ == "__main__":
    main()
