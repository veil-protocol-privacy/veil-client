use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum ProofCommands {
    Generate {
        // #[arg(short, long)]
        // token_id: String,
        // #[arg(short, long)]
        // receiver_public_viewing_key: String,
        // #[arg(short, long)]
        // proof: String,
        // #[arg(short, long)]
        // inputs: Vec<String>,
        // #[arg(short, long)]
        // outputs: Vec<String>,
        // #[arg(short, long)]
        // merkle_root: String,
        // #[arg(short, long)]
        // tree_number: u64,
        // #[arg(short, long)]
        // spending_key: String,
        // #[arg(short, long)]
        // viewing_key: String,
    },
}

impl ProofCommands {
    pub fn handle_command(command: ProofCommands) {
        todo!("handle_command: {:?}", command);
    }
}
