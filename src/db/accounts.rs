use super::{pool, ComEntry};
use crate::slint_generatedAppWindow::AccountEntry as UIAccountEntry;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub const SECRET_UUID: &str = "secret-uuid";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountEntry {
    pub uuid: String,
    pub name: String,
    pub pubkey: String,
    pub derive_index: i32,
}

impl From<UIAccountEntry> for AccountEntry {
    fn from(entry: UIAccountEntry) -> Self {
        AccountEntry {
            uuid: entry.uuid.into(),
            name: entry.name.into(),
            pubkey: entry.pubkey.into(),
            derive_index: entry.derive_index,
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
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SecretInfo {
    pub password: String,
    pub mnemonic: String,
    pub current_derive_index: i32,
}

pub async fn new() -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS accounts (
             id INTEGER PRIMARY KEY,
             uuid TEXT NOT NULL UNIQUE,
             data TEXT NOT NULL
             )",
    )
    .execute(&pool())
    .await?;

    Ok(())
}

pub async fn delete(uuid: &str) -> Result<()> {
    sqlx::query("DELETE FROM accounts WHERE uuid=?")
        .bind(uuid)
        .execute(&pool())
        .await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn delete_all() -> Result<()> {
    sqlx::query("DELETE FROM accounts").execute(&pool()).await?;
    Ok(())
}

pub async fn insert(uuid: &str, data: &str) -> Result<()> {
    sqlx::query("INSERT INTO accounts (uuid, data) VALUES (?, ?)")
        .bind(uuid)
        .bind(data)
        .execute(&pool())
        .await?;
    Ok(())
}

pub async fn update(uuid: &str, data: &str) -> Result<()> {
    sqlx::query("UPDATE accounts SET data=? WHERE uuid=?")
        .bind(data)
        .bind(uuid)
        .execute(&pool())
        .await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn select(uuid: &str) -> Result<ComEntry> {
    Ok(
        sqlx::query_as::<_, ComEntry>("SELECT * FROM accounts WHERE uuid=?")
            .bind(uuid)
            .fetch_one(&pool())
            .await?,
    )
}

pub async fn select_all() -> Result<Vec<ComEntry>> {
    Ok(sqlx::query_as::<_, ComEntry>("SELECT * FROM accounts")
        .fetch_all(&pool())
        .await?)
}

#[allow(dead_code)]
pub async fn is_exist(uuid: &str) -> Result<()> {
    select(uuid).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::sync::Mutex;

    static MTX: Mutex<()> = Mutex::new(());
    const DB_PATH: &str = "/tmp/sollet-accounts-test.db";

    #[tokio::test]
    async fn test_table_new() -> Result<()> {
        let _mtx = MTX.lock().unwrap();
        db::init(DB_PATH).await;
        new().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_all() -> Result<()> {
        let _mtx = MTX.lock().unwrap();
        db::init(DB_PATH).await;
        new().await?;
        delete_all().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_one() -> Result<()> {
        let _mtx = MTX.lock().unwrap();
        db::init(DB_PATH).await;
        new().await?;

        delete_all().await?;
        insert("uuid-1", "data-1").await?;
        delete("uuid-1").await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_insert() -> Result<()> {
        let _mtx = MTX.lock().unwrap();
        db::init(DB_PATH).await;
        new().await?;
        delete_all().await?;

        insert("uuid-1", "data-1").await?;
        insert("uuid-2", "data-2").await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_update() -> Result<()> {
        let _mtx = MTX.lock().unwrap();
        db::init(DB_PATH).await;
        new().await?;
        delete_all().await?;

        insert("uuid-1", "data-1").await?;
        update("uuid-1", "data-1-1").await?;

        assert_eq!(select("uuid-1").await?.data, "data-1-1".to_string());

        Ok(())
    }

    #[tokio::test]
    async fn test_select_one() -> Result<()> {
        let _mtx = MTX.lock().unwrap();

        db::init(DB_PATH).await;
        new().await?;
        delete_all().await?;

        assert!(select("uuid-1").await.is_err());

        insert("uuid-1", "data-1").await?;
        let item = select("uuid-1").await?;
        assert_eq!(item.uuid, "uuid-1");
        assert_eq!(item.data, "data-1");
        Ok(())
    }

    #[tokio::test]
    async fn test_select_all() -> Result<()> {
        let _mtx = MTX.lock().unwrap();

        db::init(DB_PATH).await;
        new().await?;
        delete_all().await?;

        insert("uuid-1", "data-1").await?;
        insert("uuid-2", "data-2").await?;

        let v = select_all().await?;
        assert_eq!(v[0].uuid, "uuid-1");
        assert_eq!(v[0].data, "data-1");
        assert_eq!(v[1].uuid, "uuid-2");
        assert_eq!(v[1].data, "data-2");
        Ok(())
    }

    #[tokio::test]
    async fn test_is_exist() -> Result<()> {
        let _mtx = MTX.lock().unwrap();
        db::init(DB_PATH).await;
        new().await?;
        delete_all().await?;
        insert("uuid-1", "data-1").await?;

        assert!(is_exist("uuid-0").await.is_err());
        assert!(is_exist("uuid-1").await.is_ok());
        Ok(())
    }
}
