use indexer::{client::solana::SolanaClient, event::Event};
use solana_client::nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient};
use solana_sdk::pubkey::Pubkey;
use std::{error::Error, str::FromStr};
use tokio::sync::mpsc;

// const RPC_URL: &str = "https://api.mainnet-beta.solana.com";
// const WS_URL: &str = "wss://api.mainnet-beta.solana.com/";

const RPC_URL: &str = "https://api.testnet.solana.com";
const WS_URL: &str = "wss://api.testnet.solana.com/";
const PROGRAM_ID: &str = "";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let program_id = Pubkey::from_str(PROGRAM_ID)?;

    let client = std::sync::Arc::new(SolanaClient::new(RPC_URL, WS_URL).await?);

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

    // Process received logs
    while let Some(logs) = rx.recv().await {
        for log in logs {
            if let Some(parsed_event) = Event::parse_event(&log) {
                println!("Parsed Event: {:?}", parsed_event);
                // store_event(parsed_event).await?;
            }
        }
    }

    Ok(())
}

// async fn connect_db() -> Result<Client, Box<dyn Error>> {
// let (client, connection) =
//     tokio_postgres::connect("host=localhost user=youruser dbname=yourdb password=yourpassword", NoTls).await?;
// tokio::spawn(async move {
//     if let Err(e) = connection.await {
//         eprintln!("DB Connection Error: {:?}", e);
//     }
// });
// Ok(client)
// }

// async fn store_event(event: Event) -> Result<(), Box<dyn Error>> {
// let client = connect_db().await?;
// client
//     .execute(
//         "INSERT INTO events (event_type, value) VALUES ($1, $2)",
//         &[&event.event_type, &event.value],
//     )
//     .await?;
// Ok(())
// }
