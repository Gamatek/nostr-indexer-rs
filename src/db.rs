use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::{Pool, MySql};
use std::str::FromStr;

pub async fn init_db(database_url: &str) -> Result<Pool<MySql>, sqlx::Error> {
    // Configuration de la connexion MySQL
    let connection_options = MySqlConnectOptions::from_str(database_url)?;

    // Initialisation du pool de connexions
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await?;

    // Création du schéma si la base vient d'être créée
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS torrents (
            id VARCHAR(64) PRIMARY KEY,
            infohash VARCHAR(64) NOT NULL,
            title TEXT,
            size_bytes BIGINT,
            pubkey VARCHAR(64) NOT NULL,
            created_at BIGINT NOT NULL,
            source VARCHAR(255),
            source_id VARCHAR(255),
            content TEXT,
            INDEX idx_torrents_infohash (infohash),
            INDEX idx_torrents_created_at (created_at DESC),
            INDEX idx_torrents_source (source)
        );
        "#
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS torrent_tags (
            torrent_id VARCHAR(64) NOT NULL,
            tag VARCHAR(128) NOT NULL,
            PRIMARY KEY (torrent_id, tag),
            INDEX idx_tag (tag)
        );
        "#
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS torrent_files (
            id BIGINT AUTO_INCREMENT PRIMARY KEY,
            torrent_id VARCHAR(64) NOT NULL,
            file_path TEXT NOT NULL,
            size_bytes BIGINT,
            INDEX idx_torrent_id (torrent_id)
        );
        "#
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trusted_pubkeys (
            pubkey VARCHAR(64) PRIMARY KEY,
            added_at BIGINT NOT NULL
        );
        "#
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}