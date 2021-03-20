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

/// Single transaction entry
#[derive(Debug)]
struct HistoryEntry {
    cid: u16,
    amount: Decimal,
    // Negative for withdrawal
    disputed: bool,
}

/// Internal engine implementation. Not exposed, as it is just used internally in `process` function.
#[derive(Default, Debug)]
struct Engine {
    /// Clients accounts
    clients: HashMap<u16, Client>,
    /// Transactions history
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
    fn process_transaction(&mut self, transaction: Transaction) {
        match transaction.ttype {
            TransactionType::Deposit => {
                self.process_deposit(transaction.tx, transaction.cid, transaction.amount)
            }
            TransactionType::Withdrawal => {
                self.process_whitdrawal(transaction.tx, transaction.cid, transaction.amount)
            }
            TransactionType::Dispute => self.process_dispute(transaction.tx, transaction.cid),
            TransactionType::Resolve => self.process_resolve(transaction.tx, transaction.cid),
            TransactionType::Chargeback => self.process_chargeback(transaction.tx, transaction.cid),
        }
    }

    /// Proecsses deposit transaction
    fn process_deposit(&mut self, tx: u32, cid: u16, amount: Decimal) {
        // Prevents executing same transaction twice. It is not documented, but very intuitive
        // behavior.
        if self.history.contains_key(&tx) {
            return;
        }

        let client = self.client_mut(cid);
        if !client.locked {
            client.available += amount;
            self.log(tx, cid, amount);
        }
    }

    /// Processes whithdrawal transaction
    fn process_whitdrawal(&mut self, tx: u32, cid: u16, amount: Decimal) {
        // Prevents executing same transaction twice. It is not documented, but very intuitive
        // behavior.
        if self.history.contains_key(&tx) {
            return;
        }

        let client = self.client_mut(cid);
        if client.locked || client.available >= amount {
            client.available -= amount;
            // Cannot be disputed, but for avoiding collisions
            self.log(tx, cid, -amount);
        }
    }

    /// Processes dispute transaction
    fn process_dispute(&mut self, tx: u32, cid: u16) {
        if self.client(cid).locked {
            return;
        }

        let amount = match self.history.get_mut(&tx) {
            None => return,
            // Rejects if:
            // * client id missmatches
            // * transaction amount is negative (disallow disputing withdrawal)
            // * transaction is already disputed
            Some(HistoryEntry {
                cid: tcid,
                disputed,
                amount,
            }) if *tcid != cid || *amount < Decimal::new(0, 0) || *disputed => return,
            Some(HistoryEntry {
                amount, disputed, ..
            }) => {
                *disputed = true;
                *amount
            }
        };

        let client = self.client_mut(cid);

        // This actually may put amount under 0 - for example if client deposits some money, then
        // whithdraw some of them, and then for some reason deposit is being disputes. It is not
        // clear if it is possible, but in such cases going into dept seems to be reasonable
        // solution.
        client.available -= amount;
        client.held += amount;
    }

    /// Processes resolve
    fn process_resolve(&mut self, tx: u32, cid: u16) {
        if self.client(cid).locked {
            return;
        }

        let amount = match self.history.get_mut(&tx) {
            None => return,
            // Rejects if:
            // * client id missmatches
            // * transaction is not disputed
            Some(HistoryEntry {
                cid: tcid,
                disputed,
                ..
            }) if *tcid != cid || !*disputed => return,
            Some(HistoryEntry {
                amount, disputed, ..
            }) => {
                // It is never said directly that resolved dispute makes transaction not disputed
                // anymore, but it is just logical and makes sense to me.
                *disputed = false;
                *amount
            }
        };

        let client = self.client_mut(cid);

        client.available += amount;
        client.held -= amount;
    }

    /// Process chargeback
    fn process_chargeback(&mut self, tx: u32, cid: u16) {
        if self.client(cid).locked {
            return;
        }

        let amount = match self.history.get_mut(&tx) {
            None => return,
            // Rejects if:
            // * client id missmatches
            // * transaction is not disputed
            Some(HistoryEntry {
                cid: tcid,
                disputed,
                ..
            }) if *tcid != cid || !*disputed => return,
            Some(HistoryEntry {
                amount, disputed, ..
            }) => {
                // It is never said directly that resolved dispute makes transaction not disputed
                // anymore, but it is just logical and makes sense to me.
                *disputed = false;
                *amount
            }
        };

        let client = self.client_mut(cid);

        // This should be impossible to have held being less than charged back amount, as held is
        // increased only by disputing transactions.
        assert!(client.held >= amount);
        client.held -= amount;
        client.locked = true;
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
            engine.process_transaction(transaction);
        }

        engine
    }

    #[test]
    fn dispute_siple() {
        let transactions = vec![
            Transaction {
                ttype: TransactionType::Deposit,
                cid: 1,
                tx: 1,
                amount: Decimal::new(100, 0),
            },
            Transaction {
                ttype: TransactionType::Withdrawal,
                cid: 1,
                tx: 2,
                amount: Decimal::new(50, 0),
            },
            Transaction {
                ttype: TransactionType::Deposit,
                cid: 1,
                tx: 3,
                amount: Decimal::new(200, 0),
            },
            Transaction {
                ttype: TransactionType::Dispute,
                cid: 1,
                tx: 1,
                amount: Decimal::new(0, 0),
            },
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
            Transaction {
                ttype: TransactionType::Deposit,
                cid: 1,
                tx: 1,
                amount: Decimal::new(100, 0),
            },
            Transaction {
                ttype: TransactionType::Withdrawal,
                cid: 1,
                tx: 2,
                amount: Decimal::new(50, 0),
            },
            Transaction {
                ttype: TransactionType::Deposit,
                cid: 1,
                tx: 3,
                amount: Decimal::new(200, 0),
            },
            Transaction {
                ttype: TransactionType::Withdrawal,
                cid: 1,
                tx: 4,
                amount: Decimal::new(200, 0),
            },
            Transaction {
                ttype: TransactionType::Dispute,
                cid: 1,
                tx: 1,
                amount: Decimal::new(0, 0),
            },
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
