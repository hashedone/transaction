use crate::decimal::Decimal;
use crate::transaction_type::TransactionType;
use anyhow::{anyhow, Error, Result};
use serde::Deserialize;

/// Single transaction to be performed
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "InputTransaction")]
pub struct Transaction {
    pub ttype: TransactionType,
    pub cid: u16,
    pub tx: u32,
    pub amount: Decimal,
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
        if matches!(
            ttype,
            TransactionType::Deposit | TransactionType::Withdrawal
        ) && amount.is_none()
        {
            return Err(anyhow!(
                "Missing amount on transaction, but it is required, tx: {}",
                tx
            ));
        }

        Ok(Self {
            ttype,
            cid,
            tx,
            amount: amount.unwrap_or(Decimal::new(0, 0)),
        })
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
    use crate::transaction_type::TransactionType;

    #[test]
    fn reading() {
        let data = br#"
type, client, tx, amount
deposit, 1, 1, 1.0
withdrawal, 1, 4, 1.5
dispute, 1, 5,
dispute, 1, 6,3.0"#;

        assert_eq!(
            read_transactions(&data[..])
                .map(Result::unwrap)
                .collect::<Vec<_>>(),
            vec![
                Transaction {
                    ttype: TransactionType::Deposit,
                    cid: 1,
                    tx: 1,
                    amount: Decimal::new(1, 0),
                },
                Transaction {
                    ttype: TransactionType::Withdrawal,
                    cid: 1,
                    tx: 4,
                    amount: Decimal::new(1, 5000),
                },
                Transaction {
                    ttype: TransactionType::Dispute,
                    cid: 1,
                    tx: 5,
                    amount: Decimal::new(0, 0),
                },
                Transaction {
                    ttype: TransactionType::Dispute,
                    cid: 1,
                    tx: 6,
                    amount: Decimal::new(3, 0),
                },
            ]
        );
    }
}
