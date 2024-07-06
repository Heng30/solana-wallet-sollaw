pub mod def;

pub use sqldb::{create_db, entry, ComEntry};

pub async fn init(db_path: &str) {
    create_db(db_path).await.expect("create db");
    entry::new(def::ACCOUNTS_TABLE)
        .await
        .expect("account table failed");

    entry::new(def::TOKENS_TABLE)
        .await
        .expect("tokens table failed");

    entry::new(def::ADDRESS_BOOK_TABLE)
        .await
        .expect("address_book table failed");

    entry::new(def::HISTORY_TABLE)
        .await
        .expect("history table failed");
}
