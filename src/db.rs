use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

pub async fn init_db(database_url: &str) -> Result<Pool<Sqlite>, sqlx::Error> {
    // Configuration avancée : Mode WAL activé pour meilleures perfs en lecture/écriture concurrentes,
    // plus d'autres PRAGMAs d'optimisation SQLite pour l'insertion massive et rapide
    let connection_options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .pragma("synchronous", "NORMAL") // Plus rapide que FULL, très sûr avec WAL
        .pragma("temp_store", "MEMORY")  // Utilise la RAM pour les tables temporaires et index
        .pragma("mmap_size", "30000000000") // Memory-mapped I/O (très rapide pour lire)
        .pragma("cache_size", "-10000"); // Cache de 10 MB par connexion

    // Initialisation du pool de connexions (adapté pour Tokio)
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await?;

    // Création du schéma si la base vient d'être créée
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS torrents (
            id TEXT PRIMARY KEY,
            infohash TEXT NOT NULL,
            title TEXT,
            magnet TEXT,
            size_bytes INTEGER,
            pubkey TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            tags TEXT,
            source TEXT,
            source_id TEXT,
            content TEXT,
            files TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_torrents_infohash ON torrents(infohash);
        CREATE INDEX IF NOT EXISTS idx_torrents_created_at ON torrents(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_torrents_source ON torrents(source);

        CREATE TABLE IF NOT EXISTS trusted_pubkeys (
            pubkey TEXT PRIMARY KEY,
            added_at INTEGER NOT NULL
        );
        "#
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}