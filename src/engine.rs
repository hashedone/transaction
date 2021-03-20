use crate::client::Client;
use crate::decimal::Decimal;
use crate::transaction::Transaction;
use crate::transaction_type::TransactionType;
use anyhow::Result;
use std::collections::HashMap;

/// Processes all transactions and returns input.
///
/// I actually could (and maybe should) process iterator over `Transaction` with errors already
/// handled, but I just don't want to keep all transactions in memory as it is not needed here, so
/// I went this way to achieve lazy parsing.
pub fn process(
    transactions: impl IntoIterator<Item = Result<Transaction>>,
) -> Result<impl Iterator<Item = Client>> {
    let mut engine = Engine::new();

    for transaction in transactions {
        engine.process_transaction(transaction?);
    }

    Ok(engine.into_clients())
}

/// Internal engine implementation. Not exposed, as it is just used internally in `process` function.
#[derive(Default, Debug)]
struct Engine {
    clients: HashMap<u16, Client>,
}

impl Engine {
    /// Creates new engine
    fn new() -> Self {
        Self::default()
    }

    fn client_mut(&mut self, cid: u16) -> &mut Client {
        self.clients.entry(cid).or_insert_with(|| Client::new(cid))
    }

    fn process_transaction(&mut self, transaction: Transaction) {
        match transaction.ttype {
            TransactionType::Deposit => self.process_deposit(transaction.cid, transaction.amount),
            TransactionType::Withdrawal => {
                self.process_whitdrawal(transaction.cid, transaction.amount)
            }
            _ => unimplemented!(),
        }
    }

    fn process_deposit(&mut self, cid: u16, amount: Decimal) {
        self.client_mut(cid).available += amount;
    }

    fn process_whitdrawal(&mut self, cid: u16, amount: Decimal) {
        let available = &mut self.client_mut(cid).available;
        if *available >= amount {
            *available -= amount;
        }
    }

    fn into_clients(self) -> impl Iterator<Item = Client> {
        self.clients.into_iter().map(|(_, client)| client)
    }
}
