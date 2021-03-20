use crate::decimal::Decimal;
use anyhow::{anyhow, Result};
use serde::Serialize;

/// Client info
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(into = "OutputClient")]
pub struct Client {
    pub cid: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Client {
    /// Creates new client from given id
    pub fn new(cid: u16) -> Self {
        Self {
            cid,
            available: Decimal::new(0, 0),
            held: Decimal::new(0, 0),
            locked: false,
        }
    }

    /// Returns error if client is locked
    pub fn ensure_unlocked(&self) -> Result<()> {
        if self.locked {
            Err(anyhow!("Client is locked, client id: {}", self.cid))
        } else {
            Ok(())
        }
    }
}

/// Client info ready to be stored in output
#[derive(Debug, Serialize)]
struct OutputClient {
    #[serde(rename = "client")]
    cid: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl From<Client> for OutputClient {
    fn from(
        Client {
            cid,
            available,
            held,
            locked,
        }: Client,
    ) -> OutputClient {
        Self {
            cid,
            available,
            held,
            total: available + held,
            locked,
        }
    }
}

pub fn store_clients(
    writer: impl std::io::Write,
    clients: impl IntoIterator<Item = Client>,
) -> Result<()> {
    let mut writer = csv::Writer::from_writer(writer);

    for client in clients {
        writer.serialize(client)?
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::{store_clients, Client};
    use crate::decimal::Decimal;

    #[test]
    fn store() {
        let clients = vec![
            Client {
                cid: 1,
                available: Decimal::new(1, 5000),
                held: Decimal::new(0, 0),
                locked: false,
            },
            Client {
                cid: 2,
                available: Decimal::new(2, 0),
                held: Decimal::new(0, 0),
                locked: false,
            },
        ];

        let mut buf = vec![];
        store_clients(std::io::Cursor::new(&mut buf), clients).unwrap();

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            r#"client,available,held,total,locked
1,1.5,0.0,1.5,false
2,2.0,0.0,2.0,false
"#
        );
    }
}
