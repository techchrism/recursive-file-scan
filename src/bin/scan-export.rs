use std::path::PathBuf;
use anyhow::{Context};
use clap::{Parser, ValueEnum};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

#[derive(Debug, Clone, ValueEnum)]
enum ExportType {
    JSON,
    JSONL,
    Files
}

impl ToString for ExportType {
    fn to_string(&self) -> String {
        (match self {
            ExportType::JSON => "json",
            ExportType::JSONL => "jsonl",
            ExportType::Files => "files",
        }).to_owned()
    }
}

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(value_name = "DB PATH")]
    db_path: PathBuf,

    #[arg(short, long, default_value_t = ExportType::Files)]
    export_type: ExportType
}

#[derive(Serialize)]
struct Entry {
    type_name: String,
    id: Option<u32>,
    name: String,
    path: String,
    parent: Option<u32>,
    modified: u64,
    accessed: u64,
    created: u64,
    readonly: bool,
    mode: Option<u32>,
    size: Option<u64>,
    hash: Option<String>
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let db_connection = Connection::open_with_flags(args.db_path, OpenFlags::SQLITE_OPEN_READ_ONLY).with_context(|| "Could not open database connection")?;

    let mut statement_fs_entries = db_connection.prepare(r#"
        WITH RECURSIVE entries(
            type,
            id,
            name,
            path,
            parent,
            modified,
            accessed,
            created,
            readonly,
            mode,
            size,
            hash
        ) AS (
            SELECT
                "directory" as type,
                id,
                CAST(name AS TEXT),
                CAST(name AS TEXT) as path,
                parent,
                modified,
                accessed,
                created,
                readonly,
                mode,
                NULL as size,
                NULL as hash
            FROM directories WHERE parent is NULL

            UNION ALL

            SELECT
                "file" as type,
                NULL as id,
                CAST(name AS TEXT),
                CAST(name AS TEXT) as path,
                parent,
                modified,
                accessed,
                created,
                readonly,
                mode,
                size,
                hash
            FROM files WHERE parent is NULL

            UNION ALL

            SELECT
                "directory" as type,
                directories.id,
                CAST(directories.name AS TEXT),
                entries.path || ?1 || directories.name as path,
                directories.parent,
                directories.modified,
                directories.accessed,
                directories.created,
                directories.readonly,
                directories.mode,
                NULL as size,
                NULL as hash
            FROM directories JOIN entries ON directories.parent = entries.id

            UNION ALL

            SELECT
                "file" as type,
                NULL as id,
                CAST(files.name AS TEXT),
                entries.path || ?1 || files.name as path,
                files.parent,
                files.modified,
                files.accessed,
                files.created,
                files.readonly,
                files.mode,
                files.size,
                files.hash
            FROM files JOIN entries ON files.parent = entries.id
        )

        SELECT * FROM entries
    "#)?;

    let entry_iter = statement_fs_entries.query_map((std::path::MAIN_SEPARATOR_STR,), |row| {
        let hash = row.get::<&str, Option<Vec<u8>>>("hash")?.and_then(|v|
            Some(v.iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<String>>()
                .join(""))
        );
        Ok(Entry {
            type_name: row.get("type")?,
            id: row.get("id")?,
            name: row.get("name")?,
            path: row.get("path")?,
            parent: row.get("parent")?,
            modified: row.get("modified")?,
            accessed: row.get("accessed")?,
            created: row.get("created")?,
            readonly: row.get("readonly")?,
            mode: row.get("mode")?,
            size: row.get("size")?,
            hash
        })
    })?;

    match args.export_type {
        ExportType::JSON => {
            println!("[");
            for (i, entry) in entry_iter.enumerate() {
                if i != 0 {
                    println!(",");
                }
                print!("    {}", serde_json::to_string(&entry?)?);
            }
            println!("\n]");
        },
        ExportType::JSONL => {
            for entry in entry_iter {
                println!("{}", serde_json::to_string(&entry?)?);
            }
        },
        ExportType::Files => {
            for file in entry_iter {
                let file = file?;
                if file.type_name != "file" {continue;}
                println!("{}", file.path);
            }
        },
    }

    Ok(())
}