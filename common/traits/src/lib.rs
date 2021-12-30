use common::TxHash;

pub trait SudoAccount {
    fn is_sudo(&self, account: &AccountId) -> bool;
    fn sudo(&self) -> AccountId;
}

pub trait TreasuryAccount {
    fn treasury(&self) -> AccountId;
}

pub trait TxDatabase {
    fn get_transaction(&self, hash : &TxHash) -> MorphTransaction;
}
