use anyhow::{anyhow, Result};

mod client;
mod decimal;
mod engine;
mod transaction;
mod transaction_type;

fn main() -> Result<()> {
    let path = std::env::args()
        // App name
        .skip(1)
        .next()
        .ok_or_else(|| anyhow!("Missing input file"))?;

    let transactions = transaction::read_transactions(std::fs::File::open(path)?)
        // Silently ignoring invalid transactions
        .filter_map(|t| t.ok());
    let output = engine::process(transactions)?;
    client::store_clients(std::io::stdout(), output)
}
