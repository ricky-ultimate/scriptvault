use anyhow::Result;
use chrono::Utc;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub struct User {
    pub id: String,
    pub username: String,
}

pub struct CreatedApiKey {
    pub key_id: String,
    pub plaintext: String,
}

pub async fn init_tables(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id          TEXT PRIMARY KEY,
            username    TEXT UNIQUE NOT NULL,
            created_at  TIMESTAMPTZ NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS api_keys (
            id           TEXT PRIMARY KEY,
            user_id      TEXT NOT NULL REFERENCES users(id),
            key_hash     TEXT UNIQUE NOT NULL,
            label        TEXT,
            created_at   TIMESTAMPTZ NOT NULL,
            last_used_at TIMESTAMPTZ
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS script_meta (
            id          TEXT NOT NULL,
            user_id     TEXT NOT NULL REFERENCES users(id),
            name        TEXT NOT NULL,
            version     TEXT NOT NULL,
            hash        TEXT NOT NULL,
            updated_at  TIMESTAMPTZ NOT NULL,
            PRIMARY KEY (id, user_id)
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn upsert_script_meta(
    pool: &PgPool,
    user_id: &str,
    id: &str,
    name: &str,
    version: &str,
    hash: &str,
    updated_at: chrono::DateTime<chrono::Utc>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO script_meta (id, user_id, name, version, hash, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (id, user_id) DO UPDATE
         SET name = EXCLUDED.name,
             version = EXCLUDED.version,
             hash = EXCLUDED.hash,
             updated_at = EXCLUDED.updated_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(version)
    .bind(hash)
    .bind(updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_script_meta(pool: &PgPool, user_id: &str, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM script_meta WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn script_meta_exists(pool: &PgPool, user_id: &str, id: &str) -> Result<bool> {
    let row = sqlx::query("SELECT 1 AS one FROM script_meta WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub async fn list_script_meta(pool: &PgPool, user_id: &str) -> Result<Vec<ScriptMeta>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, version, hash, updated_at FROM script_meta WHERE user_id = $1 ORDER BY updated_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|r| ScriptMeta {
            id: r.get("id"),
            name: r.get("name"),
            version: r.get("version"),
            hash: r.get("hash"),
            updated_at: r.get("updated_at"),
        })
        .collect())
}

pub struct ScriptMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub hash: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_api_key() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("sv_{}", hex::encode(bytes))
}

pub async fn find_user_by_key_hash(
    pool: &PgPool,
    key_hash: &str,
) -> Result<Option<(User, String)>> {
    let row = sqlx::query(
        "SELECT u.id AS user_id, u.username, k.id AS key_id
         FROM users u
         JOIN api_keys k ON u.id = k.user_id
         WHERE k.key_hash = $1",
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        use sqlx::Row;
        let user_id: String = r.get("user_id");
        let username: String = r.get("username");
        let key_id: String = r.get("key_id");
        (
            User {
                id: user_id,
                username,
            },
            key_id,
        )
    }))
}

pub async fn username_exists(pool: &PgPool, username: &str) -> Result<bool> {
    let row = sqlx::query("SELECT 1 AS one FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub async fn create_user(pool: &PgPool, username: &str) -> Result<(User, CreatedApiKey)> {
    let user_id = Uuid::new_v4().to_string();
    let key_id = Uuid::new_v4().to_string();
    let plaintext = generate_api_key();
    let key_hash = hash_key(&plaintext);
    let now = Utc::now();

    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO users (id, username, created_at) VALUES ($1, $2, $3)")
        .bind(&user_id)
        .bind(username)
        .bind(now)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO api_keys (id, user_id, key_hash, label, created_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&key_id)
    .bind(&user_id)
    .bind(&key_hash)
    .bind("default")
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        User {
            id: user_id,
            username: username.to_string(),
        },
        CreatedApiKey { key_id, plaintext },
    ))
}

pub async fn update_key_last_used(pool: &PgPool, key_id: &str) -> Result<()> {
    let now = Utc::now();
    sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")
        .bind(now)
        .bind(key_id)
        .execute(pool)
        .await?;
    Ok(())
}
