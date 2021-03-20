use crate::decimal::Decimal;
use crate::transaction_type::TransactionType;
use anyhow::Error;
use serde::Deserialize;

/// Single transaction to be performed
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    ttype: TransactionType,
    #[serde(rename = "client")]
    cid: u16,
    tx: u32,
    amount: Decimal,
}

/// Reads transaction from given reader
pub fn read_transactions(
    reader: impl std::io::Read,
) -> impl Iterator<Item = Result<Transaction, Error>> {
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
withdrawal, 1, 4, 1.5"#;

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
                }
            ]
        );
    }
}
