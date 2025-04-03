use solana_client::nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient};

pub mod query;
pub mod transaction;

pub struct SolanaClient {
    pub client: RpcClient,
    pub ws_client: Option<PubsubClient>,
}
