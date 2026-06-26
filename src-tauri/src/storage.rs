//! SQLite storage layer for Cryptainer.
//!
//! Only container metadata is stored in the database.
//! Encrypted blobs live on disk at blob_path.
//! This module handles all DB reads and writes — no crypto happens here.

use sqlx::SqlitePool;
use chrono::Utc;
use std::path::PathBuf;

use crate::error::{CryptoError, Result};
use crate::vault::{ContainerMeta, AuditEvent};

/// Initialize SQLite connection pool and run migrations.
pub async fn init_db(app_data_dir: &PathBuf) -> Result<SqlitePool> {
    std::fs::create_dir_all(app_data_dir)?;
    let db_path = app_data_dir.join("cryptainer.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    let pool = SqlitePool::connect(&db_url).await?;

    // Run embedded migrations from src-tauri/migrations/
    sqlx::migrate!("./migrations").run(&pool).await
        .map_err(|e| CryptoError::Database(e.to_string()))?;

    Ok(pool)
}

/// Insert a new container metadata row.
pub async fn insert_container(pool: &SqlitePool, meta: &ContainerMeta) -> Result<()> {
    let kdf_json = serde_json::to_string(&meta.kdf_params)?;
    sqlx::query(
        r#"INSERT INTO containers
           (id, name, algo, kdf, kdf_params, hint, tags, file_count, total_size,
            blob_path, blob_sha256, created_at, modified_at, format_version)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&meta.id)
    .bind(&meta.name)
    .bind(&meta.algo)
    .bind(&meta.kdf_params.kdf)
    .bind(&kdf_json)
    .bind(&meta.hint)
    .bind(&meta.tags)
    .bind(i64::try_from(meta.file_count).unwrap_or(i64::MAX))
    .bind(i64::try_from(meta.total_size).map_err(|_| CryptoError::InvalidFormat("Container size exceeds maximum representable value".into()))?)
    .bind(&meta.blob_path)
    .bind(&meta.blob_sha256)
    .bind(&meta.created_at)
    .bind(&meta.modified_at)
    .bind(meta.format_version as i64)
    .execute(pool).await?;
    Ok(())
}

/// Fetch all container metadata rows, ordered by created_at descending.
pub async fn list_containers(pool: &SqlitePool) -> Result<Vec<ContainerMeta>> {
    let rows = sqlx::query_as::<_, ContainerMetaRow>(
        r#"SELECT id, name, algo, kdf_params, hint, tags,
                  file_count, total_size, blob_path, blob_sha256,
                  created_at, modified_at, format_version
           FROM containers ORDER BY created_at DESC"#
    )
    .fetch_all(pool).await?;

    rows.into_iter().map(|r| {
        Ok(ContainerMeta {
            id: r.id,
            name: r.name,
            algo: r.algo,
            kdf_params: serde_json::from_str(&r.kdf_params)?,
            hint: r.hint,
            tags: r.tags,
            file_count: r.file_count as u32,
            total_size: r.total_size as u64,
            blob_path: r.blob_path,
            blob_sha256: r.blob_sha256,
            created_at: r.created_at,
            modified_at: r.modified_at,
            format_version: r.format_version as u8,
        })
    }).collect()
}

/// Update file_count, total_size, blob_sha256, and modified_at after a re-encrypt.
pub async fn update_container_blob(
    pool: &SqlitePool, id: &str, file_count: u32,
    total_size: u64, blob_sha256: &str
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"UPDATE containers
           SET file_count=?, total_size=?, blob_sha256=?, modified_at=?
           WHERE id=?"#,
    )
    .bind(i64::try_from(file_count).unwrap_or(i64::MAX))
    .bind(i64::try_from(total_size).map_err(|_| CryptoError::InvalidFormat("Container size exceeds maximum representable value".into()))?)
    .bind(blob_sha256)
    .bind(&now)
    .bind(id)
    .execute(pool).await?;
    Ok(())
}

/// Update the format_version for a container after v1→v2 migration.
pub async fn update_container_format_version(pool: &SqlitePool, id: &str, version: u8) -> Result<()> {
    sqlx::query("UPDATE containers SET format_version=? WHERE id=?")
        .bind(version as i64)
        .bind(id)
        .execute(pool).await?;
    Ok(())
}

/// Delete a container row by ID. Blob file deletion is handled by vault.rs.
pub async fn delete_container(pool: &SqlitePool, id: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM containers WHERE id=?")
        .bind(id)
        .execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(CryptoError::NotFound(id.to_string()));
    }
    Ok(())
}

/// Check if a container with the given name already exists.
pub async fn get_container_by_name(pool: &SqlitePool, name: &str) -> Result<Option<ContainerMeta>> {
    let r = sqlx::query_as::<_, ContainerMetaRow>(
        r#"SELECT id, name, algo, kdf_params, hint, tags,
                  file_count, total_size, blob_path, blob_sha256,
                  created_at, modified_at, format_version
           FROM containers WHERE name=?
           LIMIT 1"#
    )
    .bind(name)
    .fetch_optional(pool).await?;

    match r {
        Some(row) => Ok(Some(ContainerMeta {
            id: row.id,
            name: row.name,
            algo: row.algo,
            kdf_params: serde_json::from_str(&row.kdf_params)?,
            hint: row.hint,
            tags: row.tags,
            file_count: row.file_count as u32,
            total_size: row.total_size as u64,
            blob_path: row.blob_path,
            blob_sha256: row.blob_sha256,
            created_at: row.created_at,
            modified_at: row.modified_at,
            format_version: row.format_version as u8,
        })),
        None => Ok(None),
    }
}

/// Fetch a single container by ID.
pub async fn get_container(pool: &SqlitePool, id: &str) -> Result<ContainerMeta> {
    let r = sqlx::query_as::<_, ContainerMetaRow>(
        r#"SELECT id, name, algo, kdf_params, hint, tags,
                  file_count, total_size, blob_path, blob_sha256,
                  created_at, modified_at, format_version
           FROM containers WHERE id=?"#
    )
    .bind(id)
    .fetch_optional(pool).await?
    .ok_or_else(|| CryptoError::NotFound(id.to_string()))?;

    Ok(ContainerMeta {
        id: r.id,
        name: r.name,
        algo: r.algo,
        kdf_params: serde_json::from_str(&r.kdf_params)?,
        hint: r.hint,
        tags: r.tags,
        file_count: r.file_count as u32,
        total_size: r.total_size as u64,
        blob_path: r.blob_path,
        blob_sha256: r.blob_sha256,
        created_at: r.created_at,
        modified_at: r.modified_at,
        format_version: r.format_version as u8,
    })
}

/// Insert an audit event (best-effort observability).
/// The caller is responsible for catching failures — see `record_audit` in commands.rs.
pub async fn insert_audit_event(
    pool: &SqlitePool,
    action: &str,
    container_id: Option<&str>,
    container_name: Option<&str>,
    details: Option<&str>,
) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let ts = Utc::now().to_rfc3339();
    sqlx::query(
        r#"INSERT INTO audit_log (id, ts, action, container_id, container_name, details)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&ts)
    .bind(action)
    .bind(container_id)
    .bind(container_name)
    .bind(details)
    .execute(pool).await?;
    Ok(())
}

/// List audit events, newest first, limited to `limit` rows.
pub async fn list_audit_events(pool: &SqlitePool, limit: u32) -> Result<Vec<AuditEvent>> {
    let rows = sqlx::query_as::<_, AuditEventRow>(
        r#"SELECT id, ts, action, container_id, container_name, details
           FROM audit_log ORDER BY ts DESC LIMIT ?"#,
    )
    .bind(limit as i64)
    .fetch_all(pool).await?;

    Ok(rows.into_iter().map(|r| AuditEvent {
        id: r.id,
        ts: r.ts,
        action: r.action,
        container_id: r.container_id,
        container_name: r.container_name,
        details: r.details,
    }).collect())
}

#[derive(sqlx::FromRow)]
struct ContainerMetaRow {
    id: String,
    name: String,
    algo: String,
    kdf_params: String,
    hint: Option<String>,
    tags: Option<String>,
    file_count: i64,
    total_size: i64,
    blob_path: String,
    blob_sha256: String,
    created_at: String,
    modified_at: String,
    format_version: i64,
}

#[derive(sqlx::FromRow)]
struct AuditEventRow {
    id: String,
    ts: String,
    action: String,
    container_id: Option<String>,
    container_name: Option<String>,
    details: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn test_db() -> (SqlitePool, TempDir) {
        let dir = TempDir::new().unwrap();
        let pool = init_db(&dir.path().to_path_buf()).await.unwrap();
        (pool, dir)
    }

    #[tokio::test]
    async fn audit_event_insert_list_roundtrip() {
        let (pool, _dir) = test_db().await;
        insert_audit_event(&pool, "create", Some("c1"), Some("test-container"), None).await.unwrap();
        insert_audit_event(&pool, "delete", Some("c2"), Some("to-delete"), Some(r#"{"reason":"testing"}"#)).await.unwrap();

        let events = list_audit_events(&pool, 10).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].action, "delete");
        assert_eq!(events[0].container_name.as_deref(), Some("to-delete"));
        assert_eq!(events[1].action, "create");
        assert_eq!(events[1].container_name.as_deref(), Some("test-container"));
    }

    #[tokio::test]
    async fn audit_event_list_respects_limit() {
        let (pool, _dir) = test_db().await;
        for i in 0..5 {
            insert_audit_event(&pool, &format!("action_{}", i), None, None, None).await.unwrap();
        }
        let events = list_audit_events(&pool, 3).await.unwrap();
        assert_eq!(events.len(), 3);
    }
}
