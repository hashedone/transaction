use anyhow::{anyhow, Result};
use log::warn;

mod client;
mod decimal;
mod engine;
mod transaction;
mod transaction_type;

fn main() -> Result<()> {
    pretty_env_logger::init();

    let path = std::env::args()
        // App name
        .skip(1)
        .next()
        .ok_or_else(|| anyhow!("Missing input file"))?;

    let transactions =
        transaction::read_transactions(std::fs::File::open(path)?).filter_map(|t| match t {
            Ok(t) => Some(t),
            Err(err) => {
                warn!("Transaction parse error, rejecting: {}", err);
                None
            }
        });
    let output = engine::process(transactions)?;
    client::store_clients(std::io::stdout(), output)
}
