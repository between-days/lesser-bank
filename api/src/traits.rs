// shared trait definitions

use crate::{
    error::RepoError,
    models::{Account, NewAccount},
};

#[cfg(test)]
use mockall::{automock, predicate::*};

#[cfg_attr(test, automock)]
pub trait AccountsRepository: 'static + Sync + Send {
    fn create_account(&self, new_account: NewAccount) -> Result<Account, RepoError>;

    fn get_accounts_by_customer(&self, customer_id: i32) -> Result<Vec<Account>, RepoError>;

    fn get_account(&self, account_id: i32) -> Result<Account, RepoError>;

    fn delete_account(&self, account_id: i32) -> Result<(), RepoError>;
}
