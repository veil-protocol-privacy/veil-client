use axum::{routing::{get, post}, Router};
use base64::Engine as _;
use base64::engine::general_purpose;
use borsh::BorshDeserialize;
use indexer::{
    api_handler::handler::{leafs, roots, unspent_balances}, client::{
        solana::SolanaClient, DEPOSIT_EVENT, NULLIFIERS_EVENT, TRANSFER_EVENT, WITHDRAW_EVENT
    }, event::{
        decrypt_deposit_cipher_text, decrypt_transaction_cipher_text, get_nullifiers_from_event, Event
    }, storage::{
        db::rockdb::{RockDbStorage, StorageWrapper}, DbOptions
    }, AppState, KeyJson
};
use solana_sdk::pubkey::Pubkey;
use std::{error::Error, fs, str::FromStr, sync::RwLock};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc};

// const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
// const WS_URL: &str = "wss://api.mainnet-beta.solana.com/";

const RPC_URL: &str = "http://127.0.0.1:8899"; 
const WS_URL: &str = "ws://127.0.0.1:8900/";
const KEY_PATH: &str = "data/output.txt";
pub const PROGRAM_ID: &str = "FQXXUySvdrBWWnCsGh6vZa2benHVtvntnm6VXbFdgpV2";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let program_id = Pubkey::from_str(PROGRAM_ID)?;

    let solana_client = SolanaClient::new(RPC_URL, WS_URL);
    let client = Arc::new(solana_client.await?);
    let db_options = DbOptions::default();
    let db = match db_options.enable_merkle_indexing {
        true => Arc::new(RwLock::new(StorageWrapper::WithMerkle(
            RockDbStorage::<true>::new(&db_options.path),
        ))),
        false => Arc::new(RwLock::new(StorageWrapper::WithoutMerkle(RockDbStorage::<
            false,
        >::new(
            &db_options.path,
        )))),
    };

    let (tx, mut rx) = mpsc::channel(100);

    // Spawn WebSocket listener for real-time indexing
    tokio::spawn({
        let client = client.clone();
        let tx = tx.clone();
        async move { client.listen_to_program_logs(program_id, tx).await }
    });

    // Spawn a task for historical indexing
    // tokio::spawn({
    //     let client = client.clone();
    //     let tx = tx.clone();
    //     async move { client.fetch_historical_events(program_id, tx).await }
    // });

    drop(tx);

    let program_id = Pubkey::from_str_const(PROGRAM_ID);
    let res = fs::read_to_string("data/output.txt");

    let key = match res {
        Ok(content) => {
            let data =  general_purpose::STANDARD.decode(content).unwrap();
            let key: KeyJson = KeyJson::try_from_slice(&data).unwrap();

            key
        }
        Err(err) => {
            panic!("cannot read from file: {}", err.to_string());
        }
    };

    // Wrap in shared state
    let state = AppState { db, program_id, key, rpc_url: RPC_URL.to_string() };
    let worker_state = state.clone();

    // Spawn a task for indexer api
    tokio::spawn(async {
        // start api server
        let app = Router::new()
            .route("/root", get(roots))
            .route("/leafs", get(leafs))
            .route("/balances", post(unspent_balances))
            .with_state(state.into());

        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        let listener = TcpListener::bind(addr).await.unwrap();
        println!("Listening on {}", addr);

        axum::serve(listener, app).await.unwrap();
    });

    // Process received logs
    while let Some(logs) = rx.recv().await {
        for log in logs {
            if log.contains(&DEPOSIT_EVENT.to_string()) {
                if let Some(parsed_event) = Event::parse_event(&log) {
                    if let Ok(decoded) = general_purpose::STANDARD.decode(parsed_event.value) {
                        let (utxo, tree_num, start_position) = match decrypt_deposit_cipher_text(
                            KEY_PATH.to_string(),
                            decoded,
                        ) {
                            Ok(data) => data,
                            Err(err) => {
                                println!("error decrypting ciphertext: {}", err.to_string());
                                continue;
                            }
                        };

                        let mut db = match worker_state.db.write() {
                            Ok(data) => data,
                            Err(err) => {
                                println!("error get mutate write db: {}", err.to_string());
                                continue;
                            }
                        };

                        match db.insert_leafs(tree_num, start_position, utxo.utxo_hash()) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("error insert leafs: {}", err.to_string());
                                continue;
                            }
                        }

                        match db.insert_utxo(tree_num, start_position, utxo) {
                            Ok(_) => {}
                            Err(err) => {
                                println!("error insert leafs: {}", err.to_string());
                                continue;
                            }
                        }
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
                                Err(err) => {
                                    println!("error decrypting ciphertext: {}", err.to_string());

                                    continue;
                                }
                            };

                        println!("hey {}", utxos.len());

                        let mut db = match worker_state.db.write() {
                            Ok(data) => data,
                            Err(err) => {
                                println!("error get mutate write db: {}", err.to_string());

                                continue;
                            }
                        };

                        leafs.iter().for_each(|leaf| {
                            match db.insert_leafs(tree_num, start_position, leaf.to_vec()) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("error storing leafs: {}", err);
                                }
                            }
                        });

                        utxos.iter().for_each(|utxo| {
                            match db.insert_utxo(tree_num, start_position, utxo.clone()) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("error storing utxo: {}", err);
                                }
                            }
                        });
                    }
                }
            }

            if log.contains(&NULLIFIERS_EVENT.to_string()) {
                if let Some(parsed_event) = Event::parse_event(&log) {
                    if let Ok(decoded) = general_purpose::STANDARD.decode(parsed_event.value) {
                        let _nullifiers = match get_nullifiers_from_event(decoded) {
                            Ok(data) => data,
                            Err(_err) => continue,
                        };
                    }
                }
            }
        }
    }

    Ok(())
}
