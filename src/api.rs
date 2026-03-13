use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use crate::models::Torrent;

// Structure pour recevoir les paramètres d'URL : ?q=ubuntu
#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

// Fonction qui construit notre routeur (toutes les URLs de notre API)
pub fn create_router(pool: Pool<Sqlite>) -> Router {
    Router::new()
        .route("/api/torrents", get(search_torrents))
        .with_state(pool)
}

// Handler qui répond aux requêtes GET /api/torrents
async fn search_torrents(
    State(pool): State<Pool<Sqlite>>,
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
                magnet, 
                size_bytes, 
                pubkey, 
                created_at,
                tags,
                source,
                source_id,
                content,
                files
            FROM torrents
            WHERE title LIKE ?1
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
                magnet, 
                size_bytes, 
                pubkey, 
                created_at,
                tags,
                source,
                source_id,
                content,
                files
            FROM torrents
            ORDER BY created_at DESC
            LIMIT 50
            "#
        )
        .fetch_all(&pool)
        .await
    };

    match result {
        Ok(torrents) => Ok(Json(torrents)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Erreur de base de données : {}", e),
        )),
    }
}