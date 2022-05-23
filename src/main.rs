use std::{borrow::Cow, collections::HashMap, str::FromStr};

use csv::StringRecord;
use rust_decimal::prelude::*;

mod lib;
use lib::{handle, Transaction, TransactionKind};

// Extend StringRecord

fn parse_value<T: FromStr>(
    value: &StringRecord,
    index: usize,
    name: &str,
) -> Result<T, Cow<'static, str>> {
    match value
        .get(index)
        .ok_or(format!("could not find {}", name))?
        .trim()
        .parse::<T>()
    {
        Ok(t) => Ok(t),
        Err(_) => Err(format!("could not parse {}", name).into()),
    }
}

impl<T: FromStr> TryFrom<StringRecord> for Transaction<T> {
    type Error = Cow<'static, str>;

    fn try_from(value: StringRecord) -> Result<Self, Self::Error> {
        // Add some constants
        const KIND_INDEX: usize = 0;
        const CLIENT_INDEX: usize = 1;
        const TX_INDEX: usize = 2;
        const AMOUNT_INDEX: usize = 3;
        // Get and parse the transaction kind
        let kind_str = value.get(KIND_INDEX).ok_or(r#"could not find "type""#)?;
        // We ignore casing in case someone wrote "Deposit" instead of "deposit" and
        // such. Sadly, we cannot use a match expression for this...
        let kind = if kind_str.eq_ignore_ascii_case("deposit") {
            TransactionKind::Deposit {
                amount: parse_value::<T>(&value, AMOUNT_INDEX, "amount")?,
            }
        } else if kind_str.eq_ignore_ascii_case("withdrawal") {
            TransactionKind::Withdrawal {
                amount: parse_value::<T>(&value, AMOUNT_INDEX, "amount")?,
            }
        } else if kind_str.eq_ignore_ascii_case("dispute") {
            TransactionKind::Dispute
        } else if kind_str.eq_ignore_ascii_case("resolve") {
            TransactionKind::Resolve
        } else if kind_str.eq_ignore_ascii_case("chargeback") {
            TransactionKind::Chargeback
        } else {
            return Err(format!(r#"found unknown transaction type "{}""#, kind_str).into());
        };
        // Get and parse the client id
        let client = parse_value::<u16>(&value, CLIENT_INDEX, "client")?;
        // Get and parse the transaction id
        let tx = parse_value::<u32>(&value, TX_INDEX, "tx")?;
        Ok(Transaction::new(kind, client, tx))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get path from command line and make a reader out of it
    let path = std::env::args().nth(1).expect("input file");
    let mut rdr = csv::Reader::from_path(path).expect("could not open file");
    // Use a HashMap because we don't know if we can trust the input file
    let mut client_store = HashMap::new();
    let mut tx_store = HashMap::new();
    // Go through each record and operate on it
    for sr_result in rdr.records() {
        let tx_result: Result<Transaction<Decimal>, _> = sr_result?.try_into();
        match tx_result {
            Ok(tx) => match handle(&tx, &mut client_store, &mut tx_store) {
                _ => {} // We ignore errors for now, but they might need to be logged later
            },
            Err(_) => {} // We ignore errors for now, but they might need to be logged later
        }
    }
    // Lastly, we print the calculations
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    #[test]
    fn test_with_duplicates() -> Result<(), Box<dyn Error>> {
        let data = "
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 1, 1.0
deposit, 1, 3, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0"
            .trim();
        let mut rdr = csv::Reader::from_reader(data.as_bytes());
        let mut client_store = HashMap::new();
        let mut tx_store = HashMap::new();
        for sr_result in rdr.records() {
            let tx_result: Result<Transaction<Decimal>, _> = sr_result?.try_into();
            match tx_result {
                Ok(tx) => match handle(&tx, &mut client_store, &mut tx_store) {
                    _ => {}
                },
                Err(_) => {
                    assert!(false);
                }
            }
        }
        let client_1 = client_store.get(&1).unwrap();
        assert_eq!(client_1.available, Decimal::from_str("1.5").unwrap());
        assert_eq!(client_1.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(client_1.locked, false);
        Ok(())
    }

    #[test]
    fn test_with_resolve_and_chargeback() -> Result<(), Box<dyn Error>> {
        let data = "
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
dispute, 2, 2, 0
chargeback, 2, 2, 0
"
        .trim();
        let mut rdr = csv::Reader::from_reader(data.as_bytes());
        let mut client_store = HashMap::new();
        let mut tx_store = HashMap::new();
        for sr_result in rdr.records() {
            let tx_result: Result<Transaction<Decimal>, _> = sr_result?.try_into();
            match tx_result {
                Ok(tx) => match handle(&tx, &mut client_store, &mut tx_store) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("{}", e);
                    }
                },
                Err(_) => {
                    assert!(false);
                }
            }
        }
        let client_1 = client_store.get(&1).unwrap();
        assert_eq!(client_1.available, Decimal::from_str("1.5").unwrap());
        assert_eq!(client_1.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(client_1.locked, false);
        let client_2 = client_store.get(&2).unwrap();
        assert_eq!(client_2.locked, true);
        Ok(())
    }
}
