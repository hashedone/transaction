use anyhow::{anyhow, Error};

mod decimal;
mod transaction;
mod transaction_type;

fn main() -> Result<(), Error> {
    let path = std::env::args()
        // App name
        .skip(1)
        .next()
        .ok_or_else(|| anyhow!("Missing input file"))?;

    let tranasactions = transaction::read_transactions(std::fs::File::open(path)?);

    Ok(())
}
