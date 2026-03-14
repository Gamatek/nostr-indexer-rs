mod api;
mod db;
mod models;
mod nostr_client;

use dotenvy::dotenv;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialisation des variables d'environnement
    dotenv().ok(); 
    
    // Système de logs : S'assure que "info" est le niveau de base si aucun n'est précisé
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    info!("Démarrage de l'indexeur Nostr pour Torrents (NIP-35)...");

    // 2. Connexion et initialisation de la base de données MySQL
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "mysql://user:password@localhost/nostr_indexer".to_string());
    
    let pool = db::init_db(&db_url).await.map_err(|e| {
        error!("Erreur lors de l'initialisation de la base de données MySQL: {}", e);
        e
    })?;

    info!("Base de données MySQL connectée avec succès!");

    // Clone le pool MySQL pour qu'on puisse l'utiliser à la fois dans l'indexeur asynchrone et dans l'API
    let indexer_pool = pool.clone();
    
    // 3. Phase 2 - Configurer le client Nostr pour l'ingestion NIP-35
    // On lance la boucle d'écoute Nostr dans une tâche Tokio asynchrone
    let _indexer_task = tokio::spawn(async move {
        if let Err(e) = nostr_client::start_indexer(indexer_pool).await {
            error!("Erreur fatale dans l'indexeur Nostr : {}", e);
        }
    });
    
    // TODO: Phase 4 - Configurer le routage de l'API Axum
    let app = api::create_router(pool); // On passe la connexion SQL au framework Web

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("🚀 Serveur Web demarré ! API disponible sur http://{}", addr);
    
    // Lancement explicite du serveur Axum (qui remplace notre attente infinie précédente)
    axum::serve(listener, app).await?;
    
    Ok(())
}
