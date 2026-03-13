use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Torrent {
    pub id: String,          // ID de l'événement Nostr
    pub infohash: String,    // Tag "x" (hachage du torrent)
    pub title: Option<String>,
    pub magnet: Option<String>,
    pub size_bytes: Option<i64>,
    pub pubkey: String,      // Auteur de l'événement
    pub created_at: i64,     // Timestamp Nostr
    pub tags: Option<String>, // NOUVEAU: Liste des catégories (ex: ["film", "drame"]) en JSON
    pub source: Option<String>, // NOUVEAU: Ex: "ygg"
    pub source_id: Option<String>, // NOUVEAU: Ex: "1452654"
    pub content: Option<String>, // DESCRIPTION / IMAGE FRONT
    pub files: Option<String>, // JSON LISTE DES FICHIERS
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TrustedPubkey {
    pub pubkey: String,
    pub added_at: i64,
}