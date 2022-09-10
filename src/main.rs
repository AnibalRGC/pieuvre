use clap::Parser;
use std::fs::File;
use csv::{Reader, Writer};
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

#[derive(Parser)]
struct Args {
    file: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Transaction {
    #[serde(rename = "type")]
    transaction_type: TransactionType,

    #[serde(rename = "client")]
    client_id: u16,

    #[serde(rename(deserialize = "tx"))]
    transaction_id: u32,

    amount: Option<Decimal>,

    // bool::default is false
    #[serde(default)]
    disputed: bool,
}

#[derive(Serialize, Debug, Clone)]
struct Account {
    #[serde(rename = "client")]
    client_id: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl Account {
    fn new(client_id: u16) -> Account {
        Account {
            client_id: client_id,
            available: dec!(0),
            held: dec!(0),
            total: dec!(0),
            locked: false,
        }
    }
}

#[derive(Default, Debug)]
struct Ledger {
    transactions_by_id: HashMap<u32, Transaction>,
    account_by_id: HashMap<u16, Account>,
}

impl Ledger {
    fn process(&mut self, transaction: &Transaction) {
        match transaction.transaction_type {
            TransactionType::Deposit => {
                self.deposit(transaction);
            },
            TransactionType::Withdrawal => {
                self.withdraw(transaction);
            },
            TransactionType::Dispute => {
                self.dispute(transaction);
            },
            TransactionType::Resolve => {
                self.resolve(transaction);
            },
            TransactionType::Chargeback => {
                self.chargeback(transaction);
            },
        }
    }

    fn deposit(&mut self, transaction: &Transaction) {
        self.transactions_by_id.insert(transaction.transaction_id, transaction.clone());

        if let Some(account) = self.account_by_id.get_mut(&transaction.client_id) {
            account.available += transaction.amount.unwrap();
            account.total = account.available + account.held;
        } else {
            let mut account = Account::new(transaction.client_id);
            account.available = transaction.amount.unwrap();
            account.total = account.available;
            self.account_by_id.insert(transaction.client_id, account);
        }
    }

    fn withdraw(&mut self, transaction: &Transaction) {
        self.transactions_by_id.insert(transaction.transaction_id, transaction.clone());

        if let Some(account) = self.account_by_id.get_mut(&transaction.client_id) {
            let amount = transaction.amount.unwrap();
            if account.available < amount {
                eprintln!(
                    "Withdrawal of {} from client {} is impossible due to insufficient available funds ({})",
                    transaction.amount.unwrap(),
                    transaction.client_id,
                    account.available,
                );
            } else {
                account.available -= amount;
                account.total -= amount;
            }
        }
    }

    fn dispute(&mut self, transaction: &Transaction) {
        if let Some(fetched_transaction) = self.transactions_by_id.get_mut(&transaction.transaction_id) {
            if fetched_transaction.client_id == transaction.client_id {
                if let Some(account) = self.account_by_id.get_mut(&transaction.client_id) {
                    fetched_transaction.disputed = true;
                    let transaction_amount = fetched_transaction.amount.unwrap();
                    if account.available > transaction_amount {
                        account.available -= transaction_amount;
                        account.held += transaction_amount;
                    } else {
                        eprintln!(
                            "Dispute of {} for client {} is impossible due to unsufficient available funds ({})",
                            fetched_transaction.amount.unwrap(),
                            transaction.client_id,
                            account.available,
                        );
                    }
                }
            }
        } else {
            eprintln!("Can't find transaction id {} to dispute", transaction.transaction_id);
        }
    }

    fn resolve(&mut self, transaction: &Transaction) {
        if let Some(fetched_transaction) = self.transactions_by_id.get_mut(&transaction.transaction_id) {
            if fetched_transaction.client_id == transaction.client_id {
                if let Some(account) = self.account_by_id.get_mut(&transaction.client_id) {
                    let transaction_amount = fetched_transaction.amount.unwrap();
                    if fetched_transaction.disputed && account.held >= transaction_amount {
                            account.available += transaction_amount;
                            account.held -= transaction_amount;
                    } else {
                        eprintln!(
                            "Resolve {} for client {} is impossible due to unsufficient held funds ({}) or not disputed",
                            fetched_transaction.amount.unwrap(),
                            transaction.client_id,
                            account.held,
                        );
                    }
                    fetched_transaction.disputed = false;
                }
            }
        } else {
            eprintln!("Can't find transaction id {} to resolve", transaction.transaction_id);
        }
    }

    fn chargeback(&mut self, transaction: &Transaction) {
        if let Some(fetched_transaction) = self.transactions_by_id.get_mut(&transaction.transaction_id) {
            if fetched_transaction.client_id == transaction.client_id {
                if let Some(account) = self.account_by_id.get_mut(&transaction.client_id) {
                    let transaction_amount = fetched_transaction.amount.unwrap();
                    if fetched_transaction.disputed && account.held >= transaction_amount {
                            account.total -= transaction_amount;
                            account.held -= transaction_amount;
                            account.locked = true;
                    } else {
                        eprintln!(
                            "Chargeback {} for client {} is impossible due to unsufficient held funds ({}) or not disputed",
                            fetched_transaction.amount.unwrap(),
                            transaction.client_id,
                            account.held,
                        );
                    }
                    fetched_transaction.disputed = false;
                }
            }
        } else {
            eprintln!("Can't find transaction id {} to chargeback", transaction.transaction_id);
        }
    }

    fn get_account(&self, client_id: u16) -> Option<Account> {
        self.account_by_id.get(&client_id).cloned()
    }
}

fn main() {
    let args = Args::parse();

    let reader = File::open(&args.file)
        .map(|file| { Reader::from_reader(file) })
        .map_err(|err| {
            eprintln!("Cannot read file {} properly: {}", args.file, err);
        })
        .ok();


    let mut ledger = Ledger::default();

    for r in reader.unwrap().deserialize::<Transaction>() {
        let transaction = r.unwrap();
        ledger.process(&transaction);
    }


    let mut wrtr = Writer::from_writer(std::io::stdout());
    for account in ledger.account_by_id.values() {
        wrtr.serialize(account).unwrap();
    } 
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_test() {
        let mut ledger = Ledger::default();
        let mut transaction = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(dec!(1.5)),
            disputed: false,
        };

        ledger.deposit(&transaction);
        assert_eq!(ledger.get_account(1).unwrap().available, dec!(1.5));
        assert_eq!(ledger.get_account(1).unwrap().total, dec!(1.5));

        transaction.transaction_id = 2;
        transaction.amount = Some(dec!(4.5));

        ledger.deposit(&transaction);
        assert_eq!(ledger.get_account(1).unwrap().available, dec!(6.0));
        assert_eq!(ledger.get_account(1).unwrap().total, dec!(6.0));
    }

    #[test]
    fn withdraw_test() {
        let mut ledger = Ledger::default();
        let transaction_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(dec!(1.5)),
            disputed: false,
        };

        ledger.deposit(&transaction_deposit);

        let mut transaction_withdrawal = Transaction {
            transaction_type: TransactionType::Withdrawal,
            client_id: 1,
            transaction_id: 2,
            amount: Some(dec!(0.5)),
            disputed: false,
        };


        ledger.withdraw(&transaction_withdrawal);
        assert_eq!(ledger.get_account(1).unwrap().available, dec!(1.0));
        assert_eq!(ledger.get_account(1).unwrap().total, dec!(1.0));

        transaction_withdrawal.amount = Some(dec!(2.0));

        ledger.withdraw(&transaction_withdrawal);
        assert_eq!(ledger.get_account(1).unwrap().available, dec!(1.0));
        assert_eq!(ledger.get_account(1).unwrap().total, dec!(1.0));
    }

    #[test]
    fn dispute_test() {
        let mut ledger = Ledger::default();
        let mut transaction_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(dec!(1.5)),
            disputed: false,
        };

        ledger.deposit(&transaction_deposit);

        transaction_deposit.transaction_id = 2;
        transaction_deposit.amount = Some(dec!(10.0));

        ledger.deposit(&transaction_deposit);

        let dispute = Transaction {
            transaction_type: TransactionType::Dispute,
            client_id: 1,
            transaction_id: 1,
            amount: None,
            disputed: false,
        };

        ledger.dispute(&dispute);

        assert_eq!(ledger.get_account(1).unwrap().available, dec!(10.0));
        assert_eq!(ledger.get_account(1).unwrap().held, dec!(1.5));
        assert_eq!(ledger.get_account(1).unwrap().total, dec!(11.5));
    }

    #[test]
    fn resolve_test() {
        let mut ledger = Ledger::default();
        let mut transaction_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(dec!(1.5)),
            disputed: false,
        };

        ledger.deposit(&transaction_deposit);

        transaction_deposit.transaction_id = 2;
        transaction_deposit.amount = Some(dec!(10.0));

        ledger.deposit(&transaction_deposit);

        let dispute = Transaction {
            transaction_type: TransactionType::Dispute,
            client_id: 1,
            transaction_id: 1,
            amount: None,
            disputed: false,
        };

        ledger.dispute(&dispute);

        let resolve = Transaction {
            transaction_type: TransactionType::Resolve,
            client_id: 1,
            transaction_id: 1,
            amount: None,
            disputed: false,
        };

        ledger.resolve(&resolve);

        assert_eq!(ledger.get_account(1).unwrap().available, dec!(11.5));
        assert_eq!(ledger.get_account(1).unwrap().held, dec!(0));
        assert_eq!(ledger.get_account(1).unwrap().total, dec!(11.5));
    }

    #[test]
    fn chargeback_test() {
        let mut ledger = Ledger::default();
        let mut transaction_deposit = Transaction {
            transaction_type: TransactionType::Deposit,
            client_id: 1,
            transaction_id: 1,
            amount: Some(dec!(1.5)),
            disputed: false,
        };

        ledger.deposit(&transaction_deposit);

        transaction_deposit.transaction_id = 2;
        transaction_deposit.amount = Some(dec!(10.0));

        ledger.deposit(&transaction_deposit);

        let dispute = Transaction {
            transaction_type: TransactionType::Dispute,
            client_id: 1,
            transaction_id: 1,
            amount: None,
            disputed: false,
        };

        ledger.dispute(&dispute);

        let chargeback = Transaction {
            transaction_type: TransactionType::Chargeback,
            client_id: 1,
            transaction_id: 1,
            amount: None,
            disputed: false,
        };

        ledger.chargeback(&chargeback);

        assert_eq!(ledger.get_account(1).unwrap().available, dec!(10));
        assert_eq!(ledger.get_account(1).unwrap().held, dec!(0));
        assert_eq!(ledger.get_account(1).unwrap().total, dec!(10));
        assert!(ledger.get_account(1).unwrap().locked);
    }
}
