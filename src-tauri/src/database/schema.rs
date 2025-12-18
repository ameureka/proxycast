use rusqlite::Connection;

pub fn create_tables(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Providers 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS providers (
            id TEXT NOT NULL,
            app_type TEXT NOT NULL,
            name TEXT NOT NULL,
            settings_config TEXT NOT NULL,
            category TEXT,
            icon TEXT,
            icon_color TEXT,
            notes TEXT,
            created_at INTEGER,
            sort_index INTEGER,
            is_current INTEGER DEFAULT 0,
            PRIMARY KEY (id, app_type)
        )",
        [],
    )?;

    // MCP 服务器表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS mcp_servers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            server_config TEXT NOT NULL,
            description TEXT,
            enabled_proxycast INTEGER DEFAULT 0,
            enabled_claude INTEGER DEFAULT 0,
            enabled_codex INTEGER DEFAULT 0,
            enabled_gemini INTEGER DEFAULT 0,
            created_at INTEGER
        )",
        [],
    )?;

    // Prompts 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS prompts (
            id TEXT NOT NULL,
            app_type TEXT NOT NULL,
            name TEXT NOT NULL,
            content TEXT NOT NULL,
            description TEXT,
            enabled INTEGER DEFAULT 0,
            created_at INTEGER,
            updated_at INTEGER,
            PRIMARY KEY (id, app_type)
        )",
        [],
    )?;

    // Migration: rename is_current to enabled if old column exists
    let _ = conn.execute(
        "ALTER TABLE prompts RENAME COLUMN is_current TO enabled",
        [],
    );

    // Migration: add updated_at column if it doesn't exist
    let _ = conn.execute("ALTER TABLE prompts ADD COLUMN updated_at INTEGER", []);

    // 设置表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;

    // Skills 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS skills (
            directory TEXT NOT NULL,
            app_type TEXT NOT NULL,
            installed INTEGER NOT NULL DEFAULT 0,
            installed_at INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (directory, app_type)
        )",
        [],
    )?;

    // Skill Repos 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS skill_repos (
            owner TEXT NOT NULL,
            name TEXT NOT NULL,
            branch TEXT NOT NULL DEFAULT 'main',
            enabled INTEGER NOT NULL DEFAULT 1,
            PRIMARY KEY (owner, name)
        )",
        [],
    )?;

    // Provider Pool 凭证表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS provider_pool_credentials (
            uuid TEXT PRIMARY KEY,
            provider_type TEXT NOT NULL,
            credential_data TEXT NOT NULL,
            name TEXT,
            is_healthy INTEGER DEFAULT 1,
            is_disabled INTEGER DEFAULT 0,
            check_health INTEGER DEFAULT 1,
            check_model_name TEXT,
            not_supported_models TEXT,
            usage_count INTEGER DEFAULT 0,
            error_count INTEGER DEFAULT 0,
            last_used INTEGER,
            last_error_time INTEGER,
            last_error_message TEXT,
            last_health_check_time INTEGER,
            last_health_check_model TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    // 创建 provider_type 索引
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_provider_pool_type ON provider_pool_credentials(provider_type)",
        [],
    )?;

    // Migration: 添加 Token 缓存字段
    let _ = conn.execute(
        "ALTER TABLE provider_pool_credentials ADD COLUMN cached_access_token TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE provider_pool_credentials ADD COLUMN cached_refresh_token TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE provider_pool_credentials ADD COLUMN token_expiry_time TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE provider_pool_credentials ADD COLUMN last_refresh_time TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE provider_pool_credentials ADD COLUMN refresh_error_count INTEGER DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE provider_pool_credentials ADD COLUMN last_refresh_error TEXT",
        [],
    );

    // Migration: 添加凭证来源字段
    let _ = conn.execute(
        "ALTER TABLE provider_pool_credentials ADD COLUMN source TEXT DEFAULT 'manual'",
        [],
    );

    Ok(())
}
