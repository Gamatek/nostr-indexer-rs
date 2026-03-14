use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sqlx::{Pool, MySql};
use crate::models::Torrent;

// Structure pour recevoir les paramètres d'URL : ?q=ubuntu
#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

// Fonction qui construit notre routeur (toutes les URLs de notre API)
pub fn create_router(pool: Pool<MySql>) -> Router {
    Router::new()
        .route("/api/torrents", get(search_torrents))
        .with_state(pool)
}

// Handler qui répond aux requêtes GET /api/torrents
async fn search_torrents(
    State(pool): State<Pool<MySql>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<Torrent>>, (StatusCode, String)> {
    
    let result = if let Some(search_text) = query.q {
        let like_query = format!("%{}%", search_text);
        sqlx::query_as::<_, Torrent>(
            r#"
            SELECT 
                id, 
                infohash, 
                title, 
                size_bytes, 
                pubkey, 
                created_at,
                source,
                source_id,
                content
            FROM torrents
            WHERE title LIKE ?
            ORDER BY created_at DESC
            LIMIT 50
            "#
        )
        .bind(like_query)
        .fetch_all(&pool)
        .await
    } else {
        // Sans recherche : on affiche les 50 derniers torrents indexés
        sqlx::query_as::<_, Torrent>(
            r#"
            SELECT 
                id, 
                infohash, 
                title, 
                size_bytes, 
                pubkey, 
                created_at,
                source,
                source_id,
                content
            FROM torrents
            ORDER BY created_at DESC
            LIMIT 50
            "#
        )
        .fetch_all(&pool)
        .await
    };

    match result {
        Ok(mut torrents) => {
            // On construit le champ `magnet` programmatiquement pour chaque torrent
            for torrent in &mut torrents {
                torrent.magnet = Some(format!("magnet:?xt=urn:btih:{}", torrent.infohash));
                
                // Récupération des tags pour ce torrent
                let tags: Vec<(String,)> = sqlx::query_as("SELECT tag FROM torrent_tags WHERE torrent_id = ?")
                    .bind(&torrent.id)
                    .fetch_all(&pool)
                    .await
                    .unwrap_or_default();
                torrent.tags = tags.into_iter().map(|t| t.0).collect();

                // Récupération des fichiers pour ce torrent
                let files: Vec<crate::models::TorrentFile> = sqlx::query_as("SELECT file_path, size_bytes FROM torrent_files WHERE torrent_id = ?")
                    .bind(&torrent.id)
                    .fetch_all(&pool)
                    .await
                    .unwrap_or_default();
                torrent.files = files;
            }
            Ok(Json(torrents))
        },
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Erreur de base de données : {}", e),
        )),
    }
}