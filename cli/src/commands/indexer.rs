use clap::Subcommand;

#[derive(Clone, Debug, Subcommand)]
pub enum IndexerCommands {}

impl IndexerCommands {
    pub fn hanđle_command(command: IndexerCommands) {
        todo!("handle_command: {:?}", command);
    }
}
