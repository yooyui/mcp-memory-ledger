use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use agent_llm_mm::{
    adapters::sqlite::{
        CURRENT_DATABASE_SCHEMA_VERSION, initialize_database, inspect_database, migrate_database,
    },
    interfaces::mcp::validate_stdio_runtime,
    support::config::AppConfig,
};
use sqlx::{Connection, Row, SqliteConnection, sqlite::SqliteConnectOptions};
use tempfile::tempdir;

fn sqlite_url(path: &Path) -> String {
    format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"))
}

async fn create_legacy_database(path: &Path, valid_namespace: bool) {
    let url = sqlite_url(path);
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::from_str(&url)
            .expect("options")
            .create_if_missing(true),
    )
    .await
    .expect("legacy connection");
    sqlx::query(
        "CREATE TABLE events (event_id TEXT PRIMARY KEY, recorded_at TEXT NOT NULL, owner TEXT NOT NULL, namespace TEXT, kind TEXT NOT NULL, summary TEXT NOT NULL)",
    )
    .execute(&mut connection)
    .await
    .expect("legacy events table");
    sqlx::query(
        "INSERT INTO events (event_id, recorded_at, owner, namespace, kind, summary) VALUES ('legacy-event', '2026-01-01T00:00:00Z', 'self', ?, 'observation', 'legacy')",
    )
    .bind(if valid_namespace { "self" } else { "world" })
    .execute(&mut connection)
    .await
    .expect("legacy event");
    connection.close().await.expect("close legacy database");
}

fn backup_files(parent: &Path, database_name: &str) -> Vec<PathBuf> {
    fs::read_dir(parent)
        .expect("read parent")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(&format!("{database_name}.pre-migrate-v"))
                        && name.ends_with(".bak")
                })
        })
        .collect()
}

#[tokio::test]
async fn read_only_inspection_of_missing_database_creates_nothing() {
    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("missing-parent").join("missing.sqlite");

    let report = inspect_database(&sqlite_url(&database_path))
        .await
        .expect("read-only report");

    assert_eq!(report.status, "missing");
    assert!(!report.database_exists);
    assert!(!report.bootstrap_performed);
    assert!(!database_path.exists());
    assert!(!database_path.parent().expect("parent").exists());
}

#[tokio::test]
async fn read_only_inspection_of_legacy_database_preserves_bytes_and_schema() {
    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("legacy.sqlite");
    create_legacy_database(&database_path, true).await;
    let before = fs::read(&database_path).expect("legacy bytes");

    let report = inspect_database(&sqlite_url(&database_path))
        .await
        .expect("legacy report");

    assert_eq!(report.status, "migration_required");
    assert_eq!(report.schema_version, Some(0));
    assert!(report.migration_required);
    assert_eq!(fs::read(&database_path).expect("bytes after"), before);
    assert!(backup_files(temp.path(), "legacy.sqlite").is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn read_only_inspection_accepts_read_only_file_without_writing() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("read-only.sqlite");
    initialize_database(&sqlite_url(&database_path))
        .await
        .expect("init");
    let before = fs::read(&database_path).expect("database bytes");
    fs::set_permissions(&database_path, fs::Permissions::from_mode(0o444)).expect("make read-only");

    let report = inspect_database(&sqlite_url(&database_path))
        .await
        .expect("read-only inspection");

    assert_eq!(report.status, "current");
    assert_eq!(report.path_writable_hint, Some(false));
    assert_eq!(fs::read(&database_path).expect("bytes after"), before);
}

#[tokio::test]
async fn init_records_schema_ledger_defaults_counts_and_foreign_key_readback() {
    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("initialized.sqlite");

    let report = initialize_database(&sqlite_url(&database_path))
        .await
        .expect("init report");

    assert_eq!(report.status, "current");
    assert_eq!(report.schema_version, Some(CURRENT_DATABASE_SCHEMA_VERSION));
    assert!(report.migration_ledger_consistent);
    assert!(report.required_tables_present);
    assert!(report.runtime_defaults_present);
    assert_eq!(report.foreign_key_violations, 0);
    assert_eq!(
        report
            .table_counts
            .iter()
            .find(|count| count.table == "schema_migrations")
            .expect("ledger count")
            .rows,
        CURRENT_DATABASE_SCHEMA_VERSION
    );
    assert_eq!(report.unknown_owner_inventory.events, 0);
    assert_eq!(report.unknown_owner_inventory.claims, 0);
    assert!(!report.unknown_owner_inventory.rewrite_performed);
    assert!(
        report
            .unknown_owner_inventory
            .rewrite_requires_separate_approval
    );
}

#[tokio::test]
async fn read_only_inspection_inventories_unknown_owner_rows_without_rewriting() {
    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("unknown-inventory.sqlite");
    let url = sqlite_url(&database_path);
    initialize_database(&url).await.expect("init");

    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::from_str(&url)
            .expect("options")
            .create_if_missing(false),
    )
    .await
    .expect("connection");
    sqlx::query(
        "INSERT INTO events (event_id, recorded_at, owner, namespace, kind, summary) VALUES ('unknown-event', '2026-08-13T00:00:00Z', 'unknown', 'world', 'observation', 'legacy unknown')",
    )
    .execute(&mut connection)
    .await
    .expect("unknown event");
    sqlx::query(
        "INSERT INTO claims (claim_id, owner, namespace, subject, predicate, object, mode, status) VALUES ('unknown-claim', 'unknown', 'project/demo', 'legacy.fact', 'is', 'unreadable', 'observed', 'active')",
    )
    .execute(&mut connection)
    .await
    .expect("unknown claim");
    connection.close().await.expect("close");

    let report = inspect_database(&url).await.expect("inventory report");
    assert_eq!(report.status, "current");
    assert_eq!(report.unknown_owner_inventory.events, 1);
    assert_eq!(report.unknown_owner_inventory.claims, 1);
    assert!(!report.unknown_owner_inventory.rewrite_performed);
    assert!(
        report
            .unknown_owner_inventory
            .rewrite_requires_separate_approval
    );

    let mut connection =
        SqliteConnection::connect_with(&SqliteConnectOptions::from_str(&url).expect("options"))
            .await
            .expect("reread");
    let event_owner = sqlx::query_scalar::<_, String>(
        "SELECT owner FROM events WHERE event_id = 'unknown-event'",
    )
    .fetch_one(&mut connection)
    .await
    .expect("event owner");
    let claim_owner = sqlx::query_scalar::<_, String>(
        "SELECT owner FROM claims WHERE claim_id = 'unknown-claim'",
    )
    .fetch_one(&mut connection)
    .await
    .expect("claim owner");
    connection.close().await.expect("close reread");
    assert_eq!(event_owner, "unknown");
    assert_eq!(claim_owner, "unknown");
}

#[tokio::test]
async fn legacy_migration_creates_backup_rehearses_restore_and_preserves_rows() {
    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("legacy.sqlite");
    create_legacy_database(&database_path, true).await;

    let report = migrate_database(&sqlite_url(&database_path))
        .await
        .expect("migration");

    assert_eq!(report.status, "current");
    assert_eq!(report.restore_rehearsal, "passed_before_original_write");
    assert!(report.preserved_row_counts);
    assert_eq!(report.foreign_key_violations, 0);
    let backup_path = PathBuf::from(report.backup_path.expect("backup path"));
    assert!(backup_path.is_file());
    let event_count = report
        .table_counts
        .iter()
        .find(|count| count.table == "events")
        .expect("event count");
    assert_eq!(event_count.rows, 1);

    let backup_report = inspect_database(&sqlite_url(&backup_path))
        .await
        .expect("backup readback");
    assert_eq!(backup_report.status, "migration_required");
    assert_eq!(backup_report.schema_version, Some(0));
}

#[tokio::test]
async fn failed_rehearsal_leaves_original_recoverable_and_unmigrated() {
    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("invalid.sqlite");
    create_legacy_database(&database_path, false).await;
    let before = fs::read(&database_path).expect("legacy bytes");

    let error = migrate_database(&sqlite_url(&database_path))
        .await
        .expect_err("invalid legacy namespace must fail rehearsal");

    assert!(error.to_string().contains("CHECK constraint failed"));
    assert_eq!(fs::read(&database_path).expect("original bytes"), before);
    let report = inspect_database(&sqlite_url(&database_path))
        .await
        .expect("original readback");
    assert_eq!(report.schema_version, Some(0));
    assert_eq!(
        report
            .table_counts
            .iter()
            .find(|count| count.table == "events")
            .expect("event count")
            .rows,
        1
    );
    let backups = backup_files(temp.path(), "invalid.sqlite");
    assert_eq!(backups.len(), 1);
    assert_eq!(
        inspect_database(&sqlite_url(&backups[0]))
            .await
            .expect("backup readback")
            .schema_version,
        Some(0)
    );
}

#[tokio::test]
async fn serve_validation_refuses_missing_database_without_creating_it() {
    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("serve-missing.sqlite");
    let config = AppConfig {
        database_url: sqlite_url(&database_path),
        ..Default::default()
    };

    let error = match validate_stdio_runtime(&config).await {
        Ok(_) => panic!("serve validation must require explicit init"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("run init"));
    assert!(!database_path.exists());
}

#[tokio::test]
async fn current_database_migration_is_a_noop_without_new_backup() {
    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("current.sqlite");
    initialize_database(&sqlite_url(&database_path))
        .await
        .expect("init");
    let before = fs::read(&database_path).expect("current bytes");

    let report = migrate_database(&sqlite_url(&database_path))
        .await
        .expect("no-op migration");

    assert_eq!(report.restore_rehearsal, "not_needed_current");
    assert!(report.backup_path.is_none());
    assert_eq!(fs::read(&database_path).expect("bytes after"), before);
    assert!(backup_files(temp.path(), "current.sqlite").is_empty());
}

#[tokio::test]
async fn ledger_rows_match_declared_versions_after_migration() {
    let temp = tempdir().expect("tempdir");
    let database_path = temp.path().join("ledger.sqlite");
    create_legacy_database(&database_path, true).await;
    migrate_database(&sqlite_url(&database_path))
        .await
        .expect("migration");

    let mut connection = SqliteConnection::connect(&sqlite_url(&database_path))
        .await
        .expect("connect migrated");
    let rows = sqlx::query("SELECT version, name FROM schema_migrations ORDER BY version")
        .fetch_all(&mut connection)
        .await
        .expect("ledger rows");
    let observed = rows
        .iter()
        .map(|row| (row.get::<i64, _>("version"), row.get::<String, _>("name")))
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        vec![
            (1, "baseline_schema".to_string()),
            (2, "owner_namespace_scope".to_string()),
            (3, "reflection_audit_columns".to_string()),
        ]
    );
    connection.close().await.expect("close migrated");
}
