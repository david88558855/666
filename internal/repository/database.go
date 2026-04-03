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
	db, err := sql.Open("sqlite", dbPath+"?_journal_mode=WAL&_busy_timeout=5000&_foreign_keys=ON")
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
	if err != nil {
		return err
	}

	// 检查是否已有视频源，没有则插入默认源
	var count int
	db.QueryRow("SELECT COUNT(*) FROM video_sources").Scan(&count)
	if count == 0 {
		insertDefaultSources(db)
	}

	return nil
}

// 默认视频源
var defaultSources = []struct {
	name    string
	api     string
	detail  string
	isAdult bool
}{
	{"电影天堂资源", "http://caiji.dyttzyapi.com/api.php/provide/vod", "http://caiji.dyttzyapi.com", false},
	{"如意资源", "http://cj.rycjapi.com/api.php/provide/vod", "", false},
	{"暴风资源", "https://bfzyapi.com/api.php/provide/vod", "", false},
	{"天涯资源", "https://tyyszy.com/api.php/provide/vod", "", false},
	{"非凡影视", "http://ffzy5.tv/api.php/provide/vod", "http://ffzy5.tv", false},
	{"360资源", "https://360zy.com/api.php/provide/vod", "", false},
	{"茅台资源", "https://caiji.maotaizy.cc/api.php/provide/vod", "", false},
	{"卧龙资源", "https://wolongzyw.com/api.php/provide/vod", "", false},
	{"极速资源", "https://jszyapi.com/api.php/provide/vod", "https://jszyapi.com", false},
	{"豆瓣资源", "https://dbzy.tv/api.php/provide/vod", "", false},
	{"魔爪资源", "https://mozhuazy.com/api.php/provide/vod", "", false},
	{"魔都资源", "https://www.mdzyapi.com/api.php/provide/vod", "", false},
	{"最大资源", "https://api.zuidapi.com/api.php/provide/vod", "", false},
	{"樱花资源", "https://m3u8.apiyhzy.com/api.php/provide/vod", "", false},
	{"无尽资源", "https://api.wujinapi.me/api.php/provide/vod", "", false},
	{"旺旺短剧", "https://wwzy.tv/api.php/provide/vod", "", false},
	{"iKun资源", "https://ikunzyapi.com/api.php/provide/vod", "", false},
	{"量子资源站", "https://cj.lziapi.com/api.php/provide/vod", "", false},
	{"小猫咪资源", "https://zy.xmm.hk/api.php/provide/vod", "", false},
}

// insertDefaultSources 插入默认视频源
func insertDefaultSources(db *sql.DB) {
	stmt, err := db.Prepare(`INSERT INTO video_sources (name, api, detail, is_adult, sort_order, enabled, created_at) 
		VALUES (?, ?, ?, ?, ?, 1, datetime('now'))`)
	if err != nil {
		return
	}
	defer stmt.Close()

	for i, src := range defaultSources {
		_, _ = stmt.Exec(src.name, src.api, src.detail, src.isAdult, i)
	}
}
