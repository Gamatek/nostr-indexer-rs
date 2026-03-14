use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Torrent {
    pub id: String,          // ID de l'événement Nostr
    pub infohash: String,    // Tag "x" (hachage du torrent)
    pub title: Option<String>,
    #[sqlx(default)]
    pub magnet: Option<String>,
    pub size_bytes: Option<i64>,
    pub pubkey: String,      // Auteur de l'événement
    pub created_at: i64,     // Timestamp Nostr
    #[sqlx(skip)]
    pub tags: Vec<String>,   // NOUVEAU: Liste des catégories récupérées d'une table externe
    pub source: Option<String>, // NOUVEAU: Ex: "ygg"
    pub source_id: Option<String>, // NOUVEAU: Ex: "1452654"
    pub content: Option<String>, // DESCRIPTION / IMAGE FRONT
    #[sqlx(skip)]
    pub files: Vec<TorrentFile>, // LISTE DES FICHIERS récupérée d'une table externe
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TorrentFile {
    pub file_path: String,
    pub size_bytes: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TrustedPubkey {
    pub pubkey: String,
    pub added_at: i64,
}