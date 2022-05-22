use std::{
    collections::HashMap,
    ops::{AddAssign, SubAssign},
    str::FromStr,
};

#[derive(Debug)]
pub struct Transaction<T> {
    client: u16,
    tx: u32,
    kind: TransactionKind<T>,
}

#[derive(Debug)]
pub enum TransactionKind<T> {
    Deposit(T),
    Withdrawal(T),
    Dispute,
    Resolve,
    Chargeback,
}

impl<T> TryFrom<csv::StringRecord> for Transaction<T>
where
    T: FromStr,
{
    type Error = &'static str;

    fn try_from(value: csv::StringRecord) -> Result<Self, Self::Error> {
        // Gather values from string record
        let kind_str = value
            .get(0)
            .ok_or("could not find 'type' column")?
            .to_lowercase();
        let client = match value
            .get(1)
            .ok_or("could not find 'client' column")?
            .trim()
            .parse::<u16>()
        {
            Ok(u) => u,
            Err(_) => {
                return Err("could not parse 'client' column");
            }
        };
        let tx = match value
            .get(2)
            .ok_or("could not find 'tx' column")?
            .trim()
            .parse::<u32>()
        {
            Ok(u) => u,
            Err(_) => {
                return Err("could not parse 'tx' column");
            }
        };
        let kind = match kind_str.as_str() {
            "deposit" | "withdrawal" => {
                let amount = match value
                    .get(3)
                    .ok_or("could not find 'amount' column")?
                    .trim()
                    .parse::<T>()
                {
                    Ok(t) => t,
                    Err(_) => {
                        return Err("could not parse 'amount' column");
                    }
                };
                if kind_str.as_str() == "deposit" {
                    Deposit(amount)
                } else {
                    Withdrawal(amount)
                }
            }
            "dispute" => Dispute,
            "resolve" => Resolve,
            "chargeback" => Chargeback,
            _ => {
                return Err("found unknown transaction type");
            }
        };
        // Construct transaction
        use TransactionKind::*;
        return Ok(Transaction {
            client: client,
            tx: tx,
            kind: kind,
        });
    }
}

pub struct Client<T> {
    available: T,
    held: T,
    locked: bool,
}

impl<T> Default for Client<T>
where
    T: Default,
{
    fn default() -> Self {
        Client {
            available: T::default(),
            held: T::default(),
            locked: false,
        }
    }
}

fn handle<T>(
    tx: &Transaction<T>,
    client_store: &mut HashMap<u16, Client<T>>,
    tx_store: &mut HashMap<u32, Transaction<T>>,
) -> Result<(), &'static str>
where
    T: Default + AddAssign + SubAssign + PartialOrd + Copy,
{
    let client = client_store.entry(tx.client).or_default();
    if client.locked {
        return Err("client is locked");
    }
    use TransactionKind::*;
    match &tx.kind {
        Deposit(a) => {
            client.available += *a;
        }
        Withdrawal(a) => {
            if &client.available < a {
                return Err("not enough funds to withdraw");
            } else {
                client.available -= *a;
            }
        }
        _ => {
            let ref_tx = tx_store
                .get(&tx.tx)
                .ok_or("could not find referenced transaction")?;
            if tx.client != ref_tx.client {
                return Err("transactions are not from the same client");
            }
            return Err("todo");
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("input filepath");
    let mut rdr = csv::Reader::from_path(path).expect("could not open file");
    let mut client_store = HashMap::new();
    let mut tx_store = HashMap::new();

    for sr_result in rdr.records() {
        let tx_result: Result<Transaction<f32>, _> = sr_result?.try_into();
        match tx_result {
            Ok(tx) => match handle(&tx, &mut client_store, &mut tx_store) {
                _ => {}
            },
            Err(_) => {}
        }
    }

    println!("client, available, held, total, locked");
    for (id, client) in client_store {
        println!(
            "{}, {}, {}, {}, {}",
            id,
            client.available,
            client.held,
            client.available + client.held,
            client.locked
        );
    }

    Ok(())
}
