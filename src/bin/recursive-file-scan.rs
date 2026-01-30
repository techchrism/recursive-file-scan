use std::{collections::HashMap, os::unix::fs::PermissionsExt, path::PathBuf, time::SystemTime};

use anyhow::{Context, bail};
use clap::{Parser, crate_name, crate_version};
use rusqlite::Connection;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(value_name = "START PATH")]
    start_path: PathBuf,

    #[arg(short, long, default_value_t = false)]
    follow_symbolic_links: bool,
    #[arg(short, long)]
    export_file: Option<PathBuf>,
    #[arg(short, long, default_value_t = false)]
    verbose: bool
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let db_path = args.export_file.unwrap_or("file-scan.sqlite".into());
    if db_path.exists() {
        bail!("Export \"{}\" already exists!", db_path.display());
    }
    println!("Writing to \"{}\"...", db_path.display());
    let db_connection = Connection::open(db_path).with_context(|| "Could not open database connection")?;

    db_connection.execute_batch(
        "
        CREATE TABLE meta (
            name    TEXT NOT NULL,
            version TEXT NOT NULL
        );
        CREATE TABLE directories (
            id INTEGER PRIMARY KEY,

            parent INTEGER,
            name BLOB NOT NULL,
            modified INTEGER NOT NULL,
            accessed INTEGER NOT NULL,
            created INTEGER NOT NULL,
            readonly INTEGER NOT NULL,
            mode INTEGER,

            FOREIGN KEY (parent) REFERENCES directories (id)
        );
        CREATE TABLE files (
            parent INTEGER,
            name BLOB NOT NULL,
            modified INTEGER NOT NULL,
            accessed INTEGER NOT NULL,
            created INTEGER NOT NULL,
            readonly INTEGER NOT NULL,
            mode INTEGER,
            
            size INTEGER NOT NULL,
            hash BLOB NOT NULL,

            FOREIGN KEY (parent) REFERENCES directories (id)
        );
    ")?;
    db_connection.execute("INSERT INTO meta (name, version) VALUES (?1, ?2)", (crate_name!(), crate_version!()))?;
    
    let mut insert_directory = db_connection.prepare("
        INSERT INTO directories (
            parent, name, modified, accessed, created, readonly, mode
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7
        ) RETURNING id
    ")?;
    let mut insert_file = db_connection.prepare("
        INSERT INTO files (
            parent, name, modified, accessed, created, readonly, mode, size, hash
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9
        )
    ")?;

    let mut cached_parents = HashMap::<PathBuf, u32>::new();
    let walker = WalkDir::new(args.start_path).follow_links(args.follow_symbolic_links);
    for entry in walker {
        match entry {
            Err(e) => {
                println!("Error: {}", e);
            }
            Ok(entry) => {
                if args.verbose {
                    println!("{}", entry.path().display());
                }

                let meta = entry.metadata()?;
                let parent_id = entry.path().parent().and_then(|p| cached_parents.get(p).cloned());
                if entry.file_type().is_dir() {
                    insert_directory.query_one((
                        parent_id,
                        entry.file_name().as_encoded_bytes(),
                        meta.modified()?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs(),
                        meta.accessed()?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs(),
                        meta.created()?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs(),
                        meta.permissions().readonly(),
                        meta.permissions().mode()
                    ), |row| {
                        cached_parents.insert(entry.clone().into_path(), row.get("id")?);
                        Ok(())
                    })?;
                }
                if entry.file_type().is_file() {
                    let mut hasher = blake3::Hasher::new();
                    hasher.update_mmap(entry.path())?;
                    let hash = hasher.finalize();

                    insert_file.execute((
                        parent_id,
                        entry.file_name().as_encoded_bytes(),
                        meta.modified()?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs(),
                        meta.accessed()?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs(),
                        meta.created()?.duration_since(SystemTime::UNIX_EPOCH)?.as_secs(),
                        meta.permissions().readonly(),
                        meta.permissions().mode(),
                        meta.len(),
                        hash.as_bytes()
                    ))?;
                }
            }
        }
    }

    Ok(())
}
