use nostr_sdk::prelude::*;
use sqlx::{Pool, MySql};
use std::time::Duration;
use tracing::{info, error};

pub async fn start_indexer(db_pool: Pool<MySql>) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialisation du Client Nostr
    let my_keys = Keys::generate();
    let client = Client::new(&my_keys);

    info!("Configuration du client Nostr...");

    // 2. Ajout des relais ciblés
    // Pour commencer, on utilise des relais génériques populaires
    // À terme, on pourra ajouter des relais dédiés aux torrents/médias
    client.add_relay("wss://relay.ygg.gratis").await?;
    client.add_relay("wss://u2prelais.eliottb.dev").await?;
    client.add_relay("wss://u2p.anhkagi.net").await?;
    
    // Se connecter à tous les relais configurés
    client.connect().await;
    info!("Connecté aux relais Nostr.");

    // 2.5 Synchro Historique massive (pour attraper les X milliers de torrents du passé)
    info!("Lancement de la synchronisation de l'historique... (Cela peut prendre du temps)");
    sync_historical_torrents(&client, &db_pool).await;

    // 3. Création des Filtres (Subscriptions)
    // Torrent format: Kind 2003
    let filter_torrent = Filter::new().kind(Kind::from(2003));
    let filter_nip94 = Filter::new()
        .kind(Kind::from(1063))
        .search("magnet:"); // Filtre optionnel pour NIP-94

    // Envoyer la souscription (REQ) pour recevoir les événements
    client.subscribe(vec![filter_torrent, filter_nip94], None).await?;
    info!("Souscription aux événements NIP-35 et NIP-94 envoyée. En attente de torrents...");

    // 4. Boucle d'écoute infinie
    client.handle_notifications(|notification| async {
        if let RelayPoolNotification::Event { event, .. } = notification {
            match process_event(&event, &db_pool).await {
                Ok(true) => info!("[Nouveau Torrent indexé] ID: {}", event.id()),
                Ok(false) => {}, // Pas un torrent valide, ou pas d'infohash
                Err(e) => error!("Erreur DB lors de l'indexation de {}: {}", event.id(), e),
            }
        }
        Ok(false) // Retourne "false" pour dire de ne pas arrêter la boucle
    }).await?;

    Ok(())
}

async fn sync_historical_torrents(client: &Client, db_pool: &Pool<MySql>) {
    // Obtenir la plage de dates qu'on a déjà en base de données
    let row: (Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT MIN(created_at), MAX(created_at) FROM torrents"
    )
    .fetch_one(db_pool)
    .await
    .unwrap_or((None, None));

    let (min_ts, max_ts) = row;

    // 1. RATTRAPAGE : Si on a déjà des données, on récupère ce qu'on a raté depuis la dernière fois
    if let Some(max_val) = max_ts {
        info!("Rattrapage des torrents manqués depuis la dernière extinction...");
        let since = Timestamp::from(max_val as u64);
        let filters = vec![Filter::new().kind(Kind::from(2003)).since(since)];
        
        if let Ok(events) = client.get_events_of(filters, Some(Duration::from_secs(10))).await {
            let mut added = 0;
            for ev in &events {
                if let Ok(true) = process_event(ev, db_pool).await { 
                    added += 1; 
                }
            }
            info!("Rattrapage terminé : {} nouveaux torrents récents indexés.", added);
        }
    }

    // 2. HISTORIQUE PROFOND : On repart du plus vieux torrent connu et on remonte le temps
    let mut until = if let Some(min_val) = min_ts {
        Timestamp::from(min_val as u64)
    } else {
        Timestamp::now() // Si la BDD est vierge, on part de maintenant
    };

    info!("Lancement de la synchro historique vers le passé depuis le : {}", until.to_human_datetime());
    let mut total_indexed = 0;

    loop {
        // Filtre pour NIP-35 et NIP-94, par tranches de 2000 événements pour ne pas saturer les relais
        let filters = vec![
            Filter::new().kind(Kind::from(2003)).until(until).limit(2000),
        ];

        // On demande l'historique aux relais, avec un timeout de 30 secondes
        match client.get_events_of(filters, Some(Duration::from_secs(30))).await {
            Ok(events) => {
                if events.is_empty() {
                    info!("Synchro historique totalement terminée ! Tout le catalogue est aspiré.");
                    break;
                }

                let mut plus_vieux = until;
                let mut batch_added = 0;
                
                for event in &events {
                    // On traite et sauvegarde en base chaque événement reçu
                    match process_event(event, db_pool).await {
                        Ok(true) => {
                            total_indexed += 1;
                            batch_added += 1;
                        }
                        _ => {}
                    }
                    if event.created_at < plus_vieux {
                        plus_vieux = event.created_at;
                    }
                }

                info!("Historique : +{} torrents insérés... (Total session: {}) (Remonté jusqu'au : {})", batch_added, total_indexed, plus_vieux.to_human_datetime());

                // Condition de sortie très importante : si le relais ne renvoie plus d'événements plus vieux, ou si on est bloqués sur la même date
                if plus_vieux >= until {
                    // On force un -1 pour passer à la seconde d'avant, au risque de louper des torrents identiques de la même micro-seconde, sinon c'est la boucle infinie.
                    until = Timestamp::from(until.as_u64() - 1);
                } else {
                    until = plus_vieux;
                }
            }
            Err(e) => {
                error!("Erreur lors de la synchro historique (on reprendra plus tard) : {}", e);
                break;
            }
        }
    }
}

async fn process_event(event: &Event, pool: &Pool<MySql>) -> Result<bool, sqlx::Error> {
    // Le pubkey (Auteur de l'événement) - Pour l'instant on indexe tout, on ajoutera le WoT plus tard
    let pubkey = event.author().to_hex();
    let created_at = event.created_at().as_u64() as i64;
    let event_id = event.id().to_hex();

    // On parcourt les tags pour trouver ce qui nous intéresse
    let mut infohash = None;
    let mut title = None;
    let mut size_bytes = None;
    let mut tags_list: Vec<String> = Vec::new();
    let mut files_list: Vec<String> = Vec::new(); // Pour retenir "chemin;taille"
    let mut source = None;
    let mut source_id = None;
    
    // Le contenu ("Description", potentiellement du texte avec des images)
    let content = if event.content().is_empty() { None } else { Some(event.content().to_string()) };

    for tag in event.tags() {
        let tag_vec = tag.as_vec(); // Convertit la structure de Tag en un vec de strings
        if let Some(tag_type) = tag_vec.first() {
            let val = tag_vec.get(1).map(|s| s.to_string());
            match tag_type.as_str() {
                "x" => infohash = val, // Infohash Bittorrent
                "title" | "name" => title = val,
                "size" => {
                    if let Some(s) = val {
                        size_bytes = s.parse::<i64>().ok();
                    }
                }
                "file" => {
                    // Les fichiers individuels contenus dans le torrent
                    if let Some(f) = val {
                        files_list.push(f);
                    }
                }
                "t" => { // Les catégories (ex: "filmvidéo", "drame", "1080p")
                    if let Some(t) = val {
                        tags_list.push(t);
                    }
                }
                "l" => {
                    // Source optionnelle, par exemple "u2p.source:ygg"
                    if let Some(l) = val {
                        if l.starts_with("u2p.source:") {
                            source = Some(l.replace("u2p.source:", ""));
                        }
                    }
                }
                "i" => {
                    // ID Tracker interne, par exemple "ygg:1452654"
                    if let Some(i) = val {
                        if i.starts_with("ygg:") {
                            source_id = Some(i.replace("ygg:", ""));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Le torrent n'a pas de Hash (exigence minimale pour notre Indexeur), on l'ignore
    let hash = match infohash {
        Some(h) => h,
        None => return Ok(false),
    };

    // 1. Insertion du torrent principal
    sqlx::query(
        r#"
        INSERT INTO torrents (id, infohash, title, size_bytes, pubkey, created_at, source, source_id, content)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE 
            title = VALUES(title),
            size_bytes = VALUES(size_bytes),
            source = VALUES(source),
            source_id = VALUES(source_id),
            content = VALUES(content)
        "#
    )
    .bind(&event_id)
    .bind(hash)
    .bind(title)
    .bind(size_bytes)
    .bind(pubkey)
    .bind(created_at)
    .bind(source)
    .bind(source_id)
    .bind(content)
    .execute(pool)
    .await?;

    // 2. Insertion des tags (categories)
    for tag in tags_list {
        // Optionnel: tronquer à 128 chars pour éviter des inserts trop longs
        let truncated_tag: String = tag.chars().take(128).collect();
        sqlx::query("INSERT IGNORE INTO torrent_tags (torrent_id, tag) VALUES (?, ?)")
            .bind(&event_id)
            .bind(truncated_tag)
            .execute(pool)
            .await?;
    }

    // 3. Insertion des fichiers individuels
    for file in files_list {
        // Certains indexeurs formatent le fichier de la façon suivante : "Nom du fichier.mp4;52304895"
        let parts: Vec<&str> = file.split(';').collect();
        let file_path = parts.first().unwrap_or(&file.as_str()).to_string();
        let file_size: Option<i64> = parts.get(1).and_then(|s| s.parse().ok());
        
        sqlx::query("INSERT INTO torrent_files (torrent_id, file_path, size_bytes) VALUES (?, ?, ?)")
            .bind(&event_id)
            .bind(file_path)
            .bind(file_size)
            .execute(pool)
            .await?;
    }

    Ok(true)
}