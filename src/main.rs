use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TxType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    ChargeBack,
}

#[derive(serde::Deserialize, Debug)]
pub struct Transaction {
    #[serde(alias = "type")]
    tx_type: TxType,
    client: u16,
    tx: u32,
    amount: Option<Decimal>,
}

#[derive(serde::Serialize, Default, Clone, Debug)]
pub struct Account {
    client: u16,
    // available funds
    available: Decimal,
    // held funds
    held: Decimal,
    //total = held+available
    total: Decimal,
    // account been frozen
    locked: bool,
    //transactions that include an amount --> (txID, amount)
    #[serde(skip_serializing)]
    transactions: HashMap<u32, Decimal>,
    // IDs of tx that are under dispute
    #[serde(skip_serializing)]
    disputed: HashSet<u32>,
}
impl Account {
    pub fn new(id: u16) -> Account {
        Account {
            client: id,
            ..Default::default()
        }
    }
}

pub fn process_tx(mut accounts: HashMap<u16, Account>, tx: Transaction) -> HashMap<u16, Account> {
    if !accounts.contains_key(&tx.client) {
        accounts.insert(tx.client, Account::new(tx.client));
    }
    let account = accounts.get_mut(&tx.client).unwrap();
    match tx.tx_type {
        TxType::Deposit => {
            assert!(tx.amount.is_some(), "deposit without amount");
            // new available funds added
            account.transactions.insert(tx.tx, tx.amount.unwrap());
            account.available += tx.amount.unwrap();
            account.total += tx.amount.unwrap();
        }
        TxType::Withdrawal => {
            assert!(tx.amount.is_some(), "withdrawal without amount");
            // available funds decreased only if present
            if account.available >= tx.amount.unwrap() {
                account.available -= tx.amount.unwrap();
                account.total -= tx.amount.unwrap();
                account.transactions.insert(tx.tx, tx.amount.unwrap());
            }
        }
        TxType::Dispute => {
            // available funds decreased, held funds increased
            if let Some(amount) = account.transactions.get(&tx.tx) {
                account.available -= amount;
                account.held += amount;
                account.disputed.insert(tx.tx);
            }
        }
        TxType::Resolve => {
            // held funds decreased, available funds increased
            if account.disputed.contains(&tx.tx) {
                // if found in account.disputed, it must be in account.transactions
                let orig_amount = account.transactions.get(&tx.tx).unwrap();
                account.available += orig_amount;
                account.held -= orig_amount;
                account.disputed.retain(|tx_id| *tx_id != tx.tx);
            }
        }
        TxType::ChargeBack => {
            if account.disputed.contains(&tx.tx) {
                // if found in account.disputed, it must be in account.transactions
                let orig_amount = account.transactions.get(&tx.tx).unwrap();
                account.held -= orig_amount;
                account.total -= orig_amount;
                account.locked = true;
            }
        }
    }
    accounts
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() > 1, "please provide input file name"); // quick way to exit with an error message
    let res = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(&args[1]);
    assert!(res.is_ok(), "file does not exist");
    let mut rdr = res.unwrap();
    assert!(
        rdr.has_headers(),
        "please change input file and add an header line"
        );
    let accounts: HashMap<u16, Account> =
        rdr.deserialize().fold(HashMap::new(), |accounts, res| {
            assert!(
                res.is_ok(),
                "error in parsing a transaction record: {:?}",
                res.err()
                );
            process_tx(accounts, res.unwrap())
        });
    let mut wrt = csv::Writer::from_writer(std::io::stdout());
    for record in accounts.into_values() {
        let res = wrt.serialize(record);
        assert!(res.is_ok(), "error in writing output to stdout");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_dispute_with_missing_deposit() {
        let txs = vec![
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 1, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 2, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 3, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Dispute, tx: 5, amount: None},
        ];
        let res: HashMap<u16, Account> = txs.into_iter().fold(HashMap::new(), process_tx);
        assert_eq!(res[&1].total, dec!(3.0));
        assert_eq!(res[&1].available, dec!(3.0));
        assert_eq!(res[&1].held, dec!(0.0));
        assert!(!res[&1].locked);
    }

    #[test]
    fn test_dispute_deposit() {
        let txs = vec![
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 1, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 2, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 3, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Dispute, tx: 2, amount: None},
        ];
        let res: HashMap<u16, Account> = txs.into_iter().fold(HashMap::new(), process_tx);
        assert_eq!(res[&1].total, dec!(3.0));
        assert_eq!(res[&1].available, dec!(2.0));
        assert_eq!(res[&1].held, dec!(1.0));
        assert!(!res[&1].locked);
    }


    #[test]
    fn test_resolve_missing_dispute() {
        let txs = vec![
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 1, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 3, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Dispute, tx: 4, amount: None},
            Transaction{client: 1, tx_type: TxType::Resolve, tx: 3, amount: None},
        ];
        let res: HashMap<u16, Account> = txs.into_iter().fold(HashMap::new(), process_tx);
        assert_eq!(res[&1].total, dec!(2.0));
        assert_eq!(res[&1].available, dec!(2.0));
        assert_eq!(res[&1].held, dec!(0.0));
        assert!(!res[&1].locked);
    }

    #[test]
    fn test_resolve_dispute() {
        let txs = vec![
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 1, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 3, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Dispute, tx: 1, amount: None},
            Transaction{client: 1, tx_type: TxType::Resolve, tx: 1, amount: None},
        ];
        let res: HashMap<u16, Account> = txs.into_iter().fold(HashMap::new(), process_tx);
        assert_eq!(res[&1].total, dec!(2.0));
        assert_eq!(res[&1].available, dec!(2.0));
        assert_eq!(res[&1].held, dec!(0.0));
        assert!(!res[&1].locked);
    }

    #[test]
    fn test_chargeback_missing_dispute() {
        let txs = vec![
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 1, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 3, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Dispute, tx: 4, amount: None},
            Transaction{client: 1, tx_type: TxType::ChargeBack, tx: 3, amount: None},
        ];
        let res: HashMap<u16, Account> = txs.into_iter().fold(HashMap::new(), process_tx);
        assert_eq!(res[&1].total, dec!(2.0));
        assert_eq!(res[&1].available, dec!(2.0));
        assert_eq!(res[&1].held, dec!(0.0));
        assert!(!res[&1].locked);
    }

    #[test]
    fn test_chargeback_dispute() {
        let txs = vec![
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 1, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Deposit, tx: 3, amount: Some(dec!(1.0))},
            Transaction{client: 1, tx_type: TxType::Dispute, tx: 1, amount: None},
            Transaction{client: 1, tx_type: TxType::ChargeBack, tx: 1, amount: None},
        ];
        let res: HashMap<u16, Account> = txs.into_iter().fold(HashMap::new(), process_tx);
        assert_eq!(res[&1].total, dec!(1.0));
        assert_eq!(res[&1].available, dec!(1.0));
        assert_eq!(res[&1].held, dec!(0.0));
        assert!(res[&1].locked);
    }
}
