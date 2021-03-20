use crate::client::Client;
use crate::decimal::Decimal;
use crate::transaction::Transaction;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Helper function returning error if client ids doesn't matc,
fn cid_matches(expected: u16, occured: u16) -> Result<()> {
    if expected != occured {
        Err(anyhow!(
            "Client id doesn't match! Expected {}, but {} given",
            expected,
            occured
        ))
    } else {
        Ok(())
    }
}

/// Processes all transactions and returns input.
///
/// I actually could (and maybe should) process iterator over `Transaction` with errors already
/// handled, but I just don't want to keep all transactions in memory as it is not needed here, so
/// I went this way to achieve lazy parsing.
pub fn process(
    transactions: impl IntoIterator<Item = Transaction>,
) -> Result<impl Iterator<Item = Client>> {
    let mut engine = Engine::new();

    for transaction in transactions {
        // Invalid transactions are silently rejected
        engine.process_transaction(transaction).ok();
    }

    Ok(engine.into_clients())
}

/// Single transaction entry
#[derive(Debug)]
struct HistoryEntry {
    cid: u16,
    // Negative for withdrawal
    amount: Decimal,
    disputed: bool,
}

impl HistoryEntry {
    /// Ensures that entry is a deposit transaction, returning error otherwise
    fn ensure_deposit(&self) -> Result<()> {
        if self.amount < Decimal::new(0, 0) {
            Err(anyhow!("Transaction is not deposit"))
        } else {
            Ok(())
        }
    }

    /// Ensures that entry is disputed, returning error otherwise
    fn ensure_disputed(&self) -> Result<()> {
        if !self.disputed {
            Err(anyhow!("Transaction is not disputed"))
        } else {
            Ok(())
        }
    }

    /// Esures that entry is *not* disputed, returning error otherwise
    fn ensure_not_disputed(&self) -> Result<()> {
        if self.disputed {
            Err(anyhow!("Transaction is disputed"))
        } else {
            Ok(())
        }
    }
}

/// Internal engine implementation. Not exposed, as it is just used internally in `process` function.
#[derive(Default, Debug)]
struct Engine {
    /// Clients accounts
    clients: HashMap<u16, Client>,

    /// Transactions history
    ///
    /// Only transaction with own tx are stored (for preventing collisions, and allowing dispute).
    ///
    /// It is not clear if withdrawal transactions should be disputable, as in `Dispute`
    /// documentation it is said that founds should decrease while disputing, and actually
    /// disputing whithdraws would allow clients create temporarly money for them just on their
    /// claims, so I decided not to allot to do so, but this looks like documentation whole to me.
    ///
    /// It could be something more space efficient, but as long as transactions can not be in
    /// order, and even not every tx would be logged, this is the easiest way to handle it
    history: HashMap<u32, HistoryEntry>,
}

impl Engine {
    /// Creates new engine
    fn new() -> Self {
        Self::default()
    }

    /// Logs single transaction
    fn log(&mut self, tx: u32, cid: u16, amount: Decimal) {
        self.history.insert(
            tx,
            HistoryEntry {
                cid,
                amount,
                disputed: false,
            },
        );
    }

    /// Gives access to particular client. Adds new client if accessed for the first time.
    fn client(&mut self, cid: u16) -> &Client {
        self.clients.entry(cid).or_insert_with(|| Client::new(cid))
    }

    /// Gives mutable access to particular client. Adds new client if accessed for the first time.
    fn client_mut(&mut self, cid: u16) -> &mut Client {
        self.clients.entry(cid).or_insert_with(|| Client::new(cid))
    }

    /// Ensures, that there is no given tx in history, returning error otherwise
    fn ensure_unique(&self, tx: u32) -> Result<()> {
        if self.history.contains_key(&tx) {
            Err(anyhow!(
                "Transaction with tx which was previously resolved, tx: {}",
                tx
            ))
        } else {
            Ok(())
        }
    }

    /// Processes single transaction
    ///
    /// General thoughts:
    /// * Relative transactions (dispute/resolve/chargeback) contains client id, but it actually
    /// can be infered from transaction id (as tx is globally unique). I decided, that if those
    /// missmatch, transaction is invalid and rejected.
    /// * Transactions cannot be performed on locked accounts. They are just rejected.
    /// * Tx never colide, if they do - something went messy, transaction is rejected.
    /// * In doc there is something about freezing, but there is nothing about it anywhere else - I
    /// assume frozen == locked.
    ///
    /// Function returns `Result` when transaction is invalid and should be rejected, giving back
    /// rejection reason.
    fn process_transaction(&mut self, transaction: Transaction) -> Result<()> {
        match transaction {
            Transaction::Deposit { tx, cid, amount } => self.process_deposit(tx, cid, amount)?,
            Transaction::Withdrawal { tx, cid, amount } => {
                self.process_whitdrawal(tx, cid, amount)?
            }
            Transaction::Dispute { tx, cid } => self.process_dispute(tx, cid)?,
            Transaction::Resolve { tx, cid } => self.process_resolve(tx, cid)?,
            Transaction::Chargeback { tx, cid } => self.process_chargeback(tx, cid)?,
        }

        Ok(())
    }

    /// Processes deposit transaction
    fn process_deposit(&mut self, tx: u32, cid: u16, amount: Decimal) -> Result<()> {
        self.ensure_unique(tx)?;

        let client = self.client_mut(cid);
        client.ensure_unlocked()?;
        client.available += amount;
        self.log(tx, cid, amount);

        Ok(())
    }

    /// Processes whithdrawal transaction
    fn process_whitdrawal(&mut self, tx: u32, cid: u16, amount: Decimal) -> Result<()> {
        self.ensure_unique(tx)?;

        let client = self.client_mut(cid);
        client.ensure_unlocked()?;
        if client.available >= amount {
            client.available -= amount;
            // Cannot be disputed, but for avoiding collisions
            self.log(tx, cid, -amount);
            Ok(())
        } else {
            Err(anyhow!(
                "Trying to withdraw more than available, tx: {}, cid: {}, amount: {}",
                tx,
                cid,
                amount
            ))
        }
    }

    /// Processes dispute transaction
    fn process_dispute(&mut self, tx: u32, cid: u16) -> Result<()> {
        self.client(cid).ensure_unlocked()?;

        let amount = match self.history.get_mut(&tx) {
            None => {
                return Err(anyhow!(
                    "Transaction was not previously performed, tx: {}",
                    tx
                ))
            }
            // Rejects if:
            // * client id missmatches
            // * transaction amount is negative (disallow disputing withdrawal)
            // * transaction is already disputed
            Some(entry) => {
                cid_matches(entry.cid, cid)?;
                entry.ensure_deposit()?;
                entry.ensure_not_disputed()?;

                // Setting this should be done only after dispute is fully processed, but from this
                // point it can't fail, so this safes hash map lookup.
                entry.disputed = true;
                entry.amount
            }
        };

        let client = self.client_mut(cid);

        // This actually may put amount under 0 - for example if client deposits some money, then
        // whithdraw some of them, and then for some reason deposit is being disputes. It is not
        // clear if it is possible, but in such cases going into dept seems to be reasonable
        // solution.
        client.available -= amount;
        client.held += amount;
        Ok(())
    }

    /// Processes resolve
    fn process_resolve(&mut self, tx: u32, cid: u16) -> Result<()> {
        self.client(cid).ensure_unlocked()?;

        let amount = match self.history.get_mut(&tx) {
            None => {
                return Err(anyhow!(
                    "Transaction was not previously performed, tx: {}",
                    tx
                ))
            }
            // Rejects if:
            // * client id missmatches
            // * transaction is not disputed
            Some(entry) => {
                cid_matches(entry.cid, cid)?;
                entry.ensure_disputed()?;

                // It is never said directly that resolved dispute makes transaction not disputed
                // anymore, but it is just logical and makes sense to me.
                // Also setting this should be done only after dispute is fully processed,
                // but from this point it can't fail, so this safes hash map lookup.
                entry.disputed = false;
                entry.amount
            }
        };

        let client = self.client_mut(cid);

        client.available += amount;
        client.held -= amount;
        Ok(())
    }

    /// Process chargeback
    fn process_chargeback(&mut self, tx: u32, cid: u16) -> Result<()> {
        self.client(cid).ensure_unlocked()?;

        let amount = match self.history.get_mut(&tx) {
            None => {
                return Err(anyhow!(
                    "Transaction was not previously performed, tx: {}",
                    tx
                ))
            }
            // Rejects if:
            // * client id missmatches
            // * transaction is not disputed
            Some(entry) => {
                cid_matches(entry.cid, cid)?;
                entry.ensure_disputed()?;

                // It is never said directly that resolved dispute makes transaction not disputed
                // anymore, but it is just logical and makes sense to me.
                // Also setting this should be done only after dispute is fully processed,
                // but from this point it can't fail, so this safes hash map lookup.
                entry.disputed = false;
                entry.amount
            }
        };

        let client = self.client_mut(cid);

        // This should be impossible to have held being less than charged back amount, as held is
        // increased only by disputing transactions.
        assert!(client.held >= amount);
        client.held -= amount;
        client.locked = true;

        Ok(())
    }

    /// Converts it to clients info (for results extraction)
    fn into_clients(self) -> impl Iterator<Item = Client> {
        self.clients.into_iter().map(|(_, client)| client)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn transactions_test(transactions: impl IntoIterator<Item = Transaction>) -> Engine {
        let mut engine = Engine::new();
        for transaction in transactions {
            engine.process_transaction(transaction).ok();
        }

        engine
    }

    #[test]
    fn dispute_siple() {
        let transactions = vec![
            Transaction::Deposit {
                cid: 1,
                tx: 1,
                amount: Decimal::new(100, 0),
            },
            Transaction::Withdrawal {
                cid: 1,
                tx: 2,
                amount: Decimal::new(50, 0),
            },
            Transaction::Deposit {
                cid: 1,
                tx: 3,
                amount: Decimal::new(200, 0),
            },
            Transaction::Dispute { cid: 1, tx: 1 },
        ];

        let engine = transactions_test(transactions);
        assert_eq!(
            *engine.clients.get(&1).unwrap(),
            Client {
                cid: 1,
                available: Decimal::new(150, 0),
                held: Decimal::new(100, 0),
                locked: false,
            }
        );
    }

    #[test]
    fn dispute_into_dept() {
        let transactions = vec![
            Transaction::Deposit {
                cid: 1,
                tx: 1,
                amount: Decimal::new(100, 0),
            },
            Transaction::Withdrawal {
                cid: 1,
                tx: 2,
                amount: Decimal::new(50, 0),
            },
            Transaction::Deposit {
                cid: 1,
                tx: 3,
                amount: Decimal::new(200, 0),
            },
            Transaction::Withdrawal {
                cid: 1,
                tx: 4,
                amount: Decimal::new(200, 0),
            },
            Transaction::Dispute { cid: 1, tx: 1 },
        ];

        let engine = transactions_test(transactions);
        assert_eq!(
            *engine.clients.get(&1).unwrap(),
            Client {
                cid: 1,
                available: Decimal::new(-50, 0),
                held: Decimal::new(100, 0),
                locked: false,
            }
        );
    }
}
