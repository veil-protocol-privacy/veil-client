use axum::{Router, routing::get};
use base64::Engine as _;
use base64::engine::general_purpose;
use indexer::{
    api_handler::handler::{leafs, roots}, client::{
        solana::SolanaClient, DEPOSIT_EVENT, NULLIFIERS_EVENT, TRANSFER_EVENT, WITHDRAW_EVENT
    }, event::{
        decrypt_deposit_cipher_text, decrypt_transaction_cipher_text, get_nullifiers_from_event, Event
    }, AppState
};
use solana_sdk::pubkey::Pubkey;
use std::{error::Error, str::FromStr};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::{Mutex, mpsc},
};

// const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
// const WS_URL: &str = "wss://api.mainnet-beta.solana.com/";

const RPC_URL: &str = "https://api.testnet.solana.com";
const WS_URL: &str = "wss://api.testnet.solana.com/";
const KEY_PATH: &str = "../../../darksol-data/key";
const PROGRAM_ID: &str = "";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let program_id = Pubkey::from_str(PROGRAM_ID)?;

    let solana_client = SolanaClient::new(RPC_URL, WS_URL);
    let client = std::sync::Arc::new(solana_client.await?);

    let (tx, mut rx) = mpsc::channel(100);

    // Spawn WebSocket listener for real-time indexing
    tokio::spawn({
        let client = client.clone();
        let tx = tx.clone();
        async move { client.listen_to_program_logs(program_id, tx).await }
    });

    // Spawn a task for historical indexing
    tokio::spawn({
        let client = client.clone();
        let tx = tx.clone();
        async move { client.fetch_historical_events(program_id, tx).await }
    });

    // get initial json state
    let client_json_data = client.to_json();

    // Create shared state
    let shared_state = Arc::new(AppState {
        index: Mutex::new(client_json_data.data),
    });

    let worker_state = Arc::clone(&shared_state);

    // start api server
    let app = Router::new()
        .route("/root", get(roots))
        .route("/notes", get(leafs))
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on {}", addr);

    axum::serve(listener, app).await.unwrap();

    // Process received logs
    while let Some(logs) = rx.recv().await {
        for log in logs {
            if log.contains(&DEPOSIT_EVENT.to_string()) {
                if let Some(parsed_event) = Event::parse_event(&log) {
                    if let Ok(decoded) = general_purpose::STANDARD.decode(parsed_event.value) {
                        let (utxo, tree_num, start_position) =
                            match decrypt_deposit_cipher_text(KEY_PATH.to_string(), decoded) {
                                Ok(data) => data,
                                Err(err) => continue,
                            };

                        client.insert(vec![utxo.utxo_hash()]);
                        client.insert_utxo(leaf_index, utxo);

                        // update app state
                        update_index_state(worker_state, client.to_json().data);
                    }
                }
            }

            if log.contains(&TRANSFER_EVENT.to_string())
                || log.contains(&WITHDRAW_EVENT.to_string())
            {
                if let Some(parsed_event) = Event::parse_event(&log) {
                    if let Ok(decoded) = general_purpose::STANDARD.decode(parsed_event.value) {
                        let (utxos, leafs, tree_num, start_position) =
                            match decrypt_transaction_cipher_text(KEY_PATH.to_string(), decoded) {
                                Ok(data) => data,
                                Err(err) => continue,
                            };

                        client.insert(leafs);

                        utxos.iter().for_each(|utxo | {
                            client.insert_utxo(leaf_index, utxo.clone());
                        });

                        // update app state
                        update_index_state(worker_state, client.to_json().data);
                    }
                }
            }

            if log.contains(&NULLIFIERS_EVENT.to_string()) {
                if let Some(parsed_event) = Event::parse_event(&log) {
                    if let Ok(decoded) = general_purpose::STANDARD.decode(parsed_event.value) {
                        let nullifiers = match get_nullifiers_from_event(decoded) {
                            Ok(data) => data,
                            Err(err) => continue,
                        };
                    }
                }
            }
        }
    }

    Ok(())
}

async fn update_index_state(state: Arc<AppState>, new_data: String) {
    let mut index = state.index.lock().await;
    *index = new_data;
}
