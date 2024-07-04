use crate::slint_generatedAppWindow::{
    AccountEntry as UIAccountEntry, AddressBookEntry as UIAddressBookEntry,
    TransactionTileEntry as UIHistoryEntry, TransactionTileStatus,
};
use serde::{Deserialize, Serialize};

pub const ACCOUNTS_TABLE: &str = "accounts";
pub const ADDRESS_BOOK_TABLE: &str = "address_book";
pub const SECRET_UUID: &str = "secret-uuid";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AccountEntry {
    pub uuid: String,
    pub name: String,
    pub pubkey: String,
    pub derive_index: i32,
    pub avatar_index: i32,
    pub balance: String,
}

impl From<UIAccountEntry> for AccountEntry {
    fn from(entry: UIAccountEntry) -> Self {
        AccountEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            pubkey: entry.pubkey.into(),
            derive_index: entry.derive_index,
            avatar_index: entry.avatar_index,
            balance: entry.balance.into(),
        }
    }
}

impl From<AccountEntry> for UIAccountEntry {
    fn from(entry: AccountEntry) -> Self {
        UIAccountEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            pubkey: entry.pubkey.into(),
            derive_index: entry.derive_index,
            avatar_index: entry.avatar_index,
            balance: entry.balance.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecretInfo {
    pub password: String,
    pub mnemonic: String,
    pub current_derive_index: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AddressBookEntry {
    pub uuid: String,
    pub name: String,
    pub address: String,
}

impl From<UIAddressBookEntry> for AddressBookEntry {
    fn from(entry: UIAddressBookEntry) -> Self {
        AddressBookEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            address: entry.address.into(),
        }
    }
}

impl From<AddressBookEntry> for UIAddressBookEntry {
    fn from(entry: AddressBookEntry) -> Self {
        UIAddressBookEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            address: entry.address.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoryEntry {
    pub hash: String,
    pub balance: String,
    pub time: String,
    // TODO
    // pub status: TransactionTileStatus,
}
