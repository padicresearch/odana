#![feature(map_first_last)]

use std::sync::Arc;

use types::tx::Transaction;
use types::TxHash;

mod txlist;
mod tx_noncer;
mod tests;

type TxHashRef = Arc<TxHash>;
type TransactionRef = Arc<Transaction>;