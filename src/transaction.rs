use crate::decimal::Decimal;
use crate::transaction_type::TransactionType;
use anyhow::{anyhow, Error, Result};
use serde::Deserialize;

/// Single transaction to be performed
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "InputTransaction")]
pub enum Transaction {
    Deposit { cid: u16, tx: u32, amount: Decimal },
    Withdrawal { cid: u16, tx: u32, amount: Decimal },
    Dispute { cid: u16, tx: u32 },
    Resolve { cid: u16, tx: u32 },
    Chargeback { cid: u16, tx: u32 },
}

#[derive(Debug, Deserialize)]
pub struct InputTransaction {
    #[serde(rename = "type")]
    ttype: TransactionType,
    #[serde(rename = "client")]
    cid: u16,
    tx: u32,
    // Amount might be messing for some transactions
    amount: Option<Decimal>,
}

impl std::convert::TryFrom<InputTransaction> for Transaction {
    type Error = Error;

    fn try_from(
        InputTransaction {
            ttype,
            cid,
            tx,
            amount,
        }: InputTransaction,
    ) -> Result<Self> {
        let result = match ttype {
            TransactionType::Deposit => {
                if let Some(amount) = amount {
                    Self::Deposit { cid, tx, amount }
                } else {
                    return Err(anyhow!("Missing amount on deposit transaction, tx: {}", tx));
                }
            }
            TransactionType::Withdrawal => {
                if let Some(amount) = amount {
                    Self::Withdrawal { cid, tx, amount }
                } else {
                    return Err(anyhow!(
                        "Missing amount on withdrawal transaction, tx: {}",
                        tx
                    ));
                }
            }
            TransactionType::Dispute => Self::Dispute { cid, tx },
            TransactionType::Resolve => Self::Resolve { cid, tx },
            TransactionType::Chargeback => Self::Chargeback { cid, tx },
        };

        Ok(result)
    }
}

/// Reads transaction from given reader
pub fn read_transactions(reader: impl std::io::Read) -> impl Iterator<Item = Result<Transaction>> {
    csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader)
        .into_deserialize()
        .map(|item| item.map_err(Into::into))
}

#[cfg(test)]
mod test {
    use super::{read_transactions, Transaction};
    use crate::decimal::Decimal;

    #[test]
    fn reading() {
        let data = br#"
type, client, tx, amount
deposit, 1, 1, 1.0
withdrawal, 1, 4, 1.5
dispute, 1, 5,
dispute, 1, 6,3.0
resolve, 1, 5,
chargeback, 1, 6,"#;

        assert_eq!(
            read_transactions(&data[..])
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            vec![
                Transaction::Deposit {
                    cid: 1,
                    tx: 1,
                    amount: Decimal::new(1, 0),
                },
                Transaction::Withdrawal {
                    cid: 1,
                    tx: 4,
                    amount: Decimal::new(1, 5000),
                },
                Transaction::Dispute { cid: 1, tx: 5 },
                Transaction::Dispute { cid: 1, tx: 6 },
                Transaction::Resolve { cid: 1, tx: 5 },
                Transaction::Chargeback { cid: 1, tx: 6 },
            ]
        );
    }
}
