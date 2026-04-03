package repository

import (
	"database/sql"
	"fmt"
	"os"
	"path/filepath"

	_ "modernc.org/sqlite"
)

// NewDB 初始化数据库
func NewDB(dataDir string) (*sql.DB, error) {
	// 确保目录存在
	if err := os.MkdirAll(dataDir, 0755); err != nil {
		return nil, fmt.Errorf("创建数据目录失败: %w", err)
	}

	dbPath := filepath.Join(dataDir, "katelyatv.db")
	db, err := sql.Open("sqlite3", dbPath+"?_journal_mode=WAL&_busy_timeout=5000&_foreign_keys=ON")
	if err != nil {
		return nil, fmt.Errorf("打开数据库失败: %w", err)
	}

	// 设置连接池
	db.SetMaxOpenConns(1)
	db.SetMaxIdleConns(1)

	// 初始化表结构
	if err := initSchema(db); err != nil {
		db.Close()
		return nil, fmt.Errorf("初始化表结构失败: %w", err)
	}

	return db, nil
}

// initSchema 初始化表结构
func initSchema(db *sql.DB) error {
	schema := `
	-- 用户表
	CREATE TABLE IF NOT EXISTS users (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		username TEXT UNIQUE NOT NULL,
		password_hash TEXT NOT NULL,
		is_admin INTEGER DEFAULT 0,
		is_adult INTEGER DEFAULT 0,
		theme TEXT DEFAULT 'auto',
		layout TEXT DEFAULT 'grid',
		items_per_page INTEGER DEFAULT 24,
		created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
		updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
	);

	-- 视频源表
	CREATE TABLE IF NOT EXISTS video_sources (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		name TEXT NOT NULL,
		api TEXT NOT NULL,
		detail TEXT DEFAULT '',
		is_adult INTEGER DEFAULT 0,
		sort_order INTEGER DEFAULT 0,
		enabled INTEGER DEFAULT 1,
		created_at DATETIME DEFAULT CURRENT_TIMESTAMP
	);

	-- 收藏表
	CREATE TABLE IF NOT EXISTS favorites (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		user_id INTEGER NOT NULL,
		site TEXT NOT NULL,
		site_name TEXT NOT NULL,
		video_id TEXT NOT NULL,
		title TEXT NOT NULL,
		cover TEXT DEFAULT '',
		type TEXT DEFAULT 'movie',
		created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
		FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
		UNIQUE(user_id, site, video_id)
	);

	-- 历史记录表
	CREATE TABLE IF NOT EXISTS history (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		user_id INTEGER NOT NULL,
		site TEXT NOT NULL,
		site_name TEXT NOT NULL,
		video_id TEXT NOT NULL,
		title TEXT NOT NULL,
		cover TEXT DEFAULT '',
		type TEXT DEFAULT 'movie',
		episode INTEGER DEFAULT 1,
		progress INTEGER DEFAULT 0,
		duration INTEGER DEFAULT 0,
		updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
		FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
		UNIQUE(user_id, site, video_id, episode)
	);

	-- 设置表
	CREATE TABLE IF NOT EXISTS settings (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		key TEXT UNIQUE NOT NULL,
		value TEXT NOT NULL
	);

	-- 缓存表
	CREATE TABLE IF NOT EXISTS cache (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		key TEXT UNIQUE NOT NULL,
		value TEXT NOT NULL,
		expired_at INTEGER NOT NULL
	);

	-- 创建索引
	CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
	CREATE INDEX IF NOT EXISTS idx_favorites_user_id ON favorites(user_id);
	CREATE INDEX IF NOT EXISTS idx_history_user_id ON history(user_id);
	CREATE INDEX IF NOT EXISTS idx_cache_key ON cache(key);
	CREATE INDEX IF NOT EXISTS idx_cache_expired ON cache(expired_at);
	`

	_, err := db.Exec(schema)
	return err
}
