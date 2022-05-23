use core::ops::{AddAssign, SubAssign};
use std::{borrow::Cow, collections::HashMap};

// Transaction
#[derive(Debug, Clone, Copy)]
pub struct Transaction<T> {
    pub kind: TransactionKind<T>,
    pub client: u16,
    pub tx: u32,
    status: TransactionStatus,
}

#[derive(Debug, Clone, Copy)]
pub enum TransactionKind<T> {
    Deposit { amount: T },
    Withdrawal { amount: T },
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone, Copy)]
enum TransactionStatus {
    Started,
    Disputed,
    Resolved,
    Chargeback,
}

impl<T> Transaction<T> {
    pub fn new(kind: TransactionKind<T>, client: u16, tx: u32) -> Self {
        Self {
            kind,
            client,
            tx,
            status: TransactionStatus::Started,
        }
    }
}

// Client
#[derive(Debug)]
pub struct Client<T> {
    pub available: T,
    pub held: T,
    pub locked: bool,
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

// Transaction Handler
pub fn handle<T>(
    tx: &Transaction<T>,
    client_store: &mut HashMap<u16, Client<T>>,
    tx_store: &mut HashMap<u32, Transaction<T>>,
) -> Result<(), Cow<'static, str>>
where
    T: Default + AddAssign + SubAssign + PartialOrd + Copy + std::fmt::Debug,
{
    // Get the client or create a new one if it doesn't exist
    let client = client_store.entry(tx.client).or_default();
    // If the client is locked, we can't really do anything with them
    if client.locked {
        return Err("client is locked".into());
    }
    // Process the transaction
    use TransactionKind::*;
    match &tx.kind {
        // We might not need to check anything when depositing money
        Deposit { amount } => {
            // Skip duplicate transactions
            if tx_store.get(&tx.tx).is_some() {
                return Err(format!("found duplicate transaction {}", tx.tx).into());
            }
            client.available += *amount;
            tx_store.insert(tx.tx, *tx);
        }
        // When withdrawing money, we need to make sure there's enough money to withdraw
        Withdrawal { amount } => {
            // Skip duplicate transactions
            if tx_store.get(&tx.tx).is_some() {
                return Err(format!("found duplicate transaction {}", tx.tx).into());
            }
            if &client.available < amount {
                return Err("not enough funds to withdraw".into());
            } else {
                client.available -= *amount;
            }
            tx_store.insert(tx.tx, *tx);
        }
        // All other cases reference a transaction, so we might reuse some code
        _ => {
            // First we try to find the transaction, and return an error if it doesn't exist
            let ref_tx = tx_store.get(&tx.tx).ok_or(format!(
                r#"could not find referenced transaction "{}""#,
                tx.tx
            ))?;
            // I don't think a client should be able to deal with other clients'
            // transactions
            if tx.client != ref_tx.client {
                return Err("transactions are not from the same client".into());
            }
            // Deal with a dispute
            if matches!(tx.kind, Dispute) {
                // I don't think we should allow a transaction to be disputed twice
                if matches!(ref_tx.status, TransactionStatus::Disputed) {
                    return Err(format!(r#"transaction "{}" already in dispute"#, tx.tx).into());
                }
                // Likewise, we should not be able to re-dispute a transaction that has been
                // resolved
                if matches!(ref_tx.status, TransactionStatus::Resolved)
                    || matches!(ref_tx.status, TransactionStatus::Chargeback)
                {
                    return Err(format!(r#"transaction "{}" already resolved"#, tx.tx).into());
                }
                // Also, a dispute needs to specify a transaction with an amount
                match ref_tx.kind {
                    Deposit { amount } | Withdrawal { amount } => {
                        // Update transaction status and client information
                        tx_store
                            .entry(tx.tx)
                            .and_modify(|t| t.status = TransactionStatus::Disputed);
                        // XXX: Can a client's available amount go under 0?
                        client.available -= amount;
                        client.held += amount;
                    }
                    _ => {
                        return Err(
                            format!(r#"transaction "{}" does not have an amount"#, tx.tx).into(),
                        );
                    }
                }
            // Deal with a resolve
            } else if matches!(tx.kind, Resolve) {
                // We can only resolve a transaction in dispute
                if !matches!(ref_tx.status, TransactionStatus::Disputed) {
                    return Err(format!(r#"transaction "{}" is not in dispute"#, tx.tx).into());
                }
                // Also, a resolve needs to specify a transaction with an amount
                match ref_tx.kind {
                    Deposit { amount } | Withdrawal { amount } => {
                        // Update transaction status and client information
                        tx_store
                            .entry(tx.tx)
                            .and_modify(|t| t.status = TransactionStatus::Resolved);
                        // XXX: Can held go under 0?
                        client.available += amount;
                        client.held -= amount;
                    }
                    _ => {
                        return Err(
                            format!(r#"transaction "{}" does not have an amount"#, tx.tx).into(),
                        );
                    }
                }
            } else {
                // We can only resolve a transaction in dispute or resolved
                if !matches!(ref_tx.status, TransactionStatus::Disputed)
                    && !matches!(ref_tx.status, TransactionStatus::Resolved)
                {
                    return Err(
                        format!(r#"transaction "{}" is not in dispute/resolved"#, tx.tx).into(),
                    );
                }
                // Also, a chargeback needs to specify a transaction with an amount
                match ref_tx.kind {
                    Deposit { amount } | Withdrawal { amount } => {
                        // Update transaction status and client information
                        tx_store
                            .entry(tx.tx)
                            .and_modify(|t| t.status = TransactionStatus::Chargeback);
                        // XXX: Can held go under 0?
                        client.held -= amount;
                        client.locked = true;
                    }
                    _ => {
                        return Err(
                            format!(r#"transaction "{}" does not have an amount"#, tx.tx).into(),
                        );
                    }
                }
            }
        }
    }
    // After all is said and done, we can add this transaction to the record
    Ok(())
}
