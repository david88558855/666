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

	-- 视频源表 (omnibox 格式)
	CREATE TABLE IF NOT EXISTS video_sources (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		key TEXT NOT NULL,
		name TEXT NOT NULL,
		api TEXT NOT NULL,
		type INTEGER DEFAULT 2,
		is_active INTEGER DEFAULT 1,
		is_default INTEGER DEFAULT 0,
		remark TEXT DEFAULT '',
		tags TEXT DEFAULT '',
		priority INTEGER DEFAULT 0,
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

// OmniboxSource omnibox 格式的视频源
type OmniboxSource struct {
	Key      string
	Name     string
	API      string
	Type     int
	IsActive bool
	Tags     string
	Priority int
}

// 默认视频源 (来自 omnibox 影视源)
var defaultSources = []OmniboxSource{
	{"豪华资源", "豪华资源", "https://hhzyapi.com/api.php/provide/vod/", 2, true, "优秀", 1},
	{"非凡影视", "非凡影视", "http://ffzy5.tv/api.php/provide/vod/", 2, true, "优秀", 0},
	{"如意资源", "如意资源", "http://cj.rycjapi.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"电影天堂资源", "电影天堂资源", "https://caiji.dyttzyapi.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"暴风资源", "暴风资源", "https://bfzyapi.com/api.php/provide/vod/", 2, true, "", 0},
	{"天涯资源", "天涯资源", "https://tyyszy.com/api.php/provide/vod/", 2, true, "", 0},
	{"360资源", "360资源", "https://360zy.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"豆瓣资源", "豆瓣资源", "https://dbzy.tv/api.php/provide/vod/", 2, true, "优秀", 0},
	{"魔爪资源", "魔爪资源", "https://mozhuazy.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"魔都资源", "魔都资源", "https://www.mdzyapi.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"最大资源", "最大资源", "https://api.zuidapi.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"樱花资源", "樱花资源", "https://m3u8.apiyhzy.com/api.php/provide/vod/", 2, true, "", 0},
	{"百度云资源", "百度云资源", "https://api.apibdzy.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"无尽资源", "无尽资源", "https://api.wujinapi.me/api.php/provide/vod/", 2, true, "", 0},
	{"旺旺短剧", "旺旺短剧", "https://wwzy.tv/api.php/provide/vod/", 2, true, "优秀", 0},
	{"iKun资源", "iKun资源", "https://ikunzyapi.com/api.php/provide/vod/", 2, true, "", 0},
	{"量子资源站", "量子资源站", "https://cj.lziapi.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"茅台资源", "茅台资源", "https://caiji.maotaizy.cc/api.php/provide/vod/", 2, true, "优秀", 0},
	{"卧龙资源2", "卧龙资源2", "https://collect.wolongzyw.com/api.php/provide/vod/", 2, true, "", 0},
	{"速播资源", "速播资源", "https://subocaiji.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"索尼资源", "索尼资源", "https://suoniapi.com/api.php/provide/vod/", 2, true, "", 0},
	{"虎牙资源", "虎牙资源", "https://www.huyaapi.com/api.php/provide/vod/", 2, true, "", 0},
	{"金鹰资源", "金鹰资源", "https://jyzyapi.com/api.php/provide/vod/", 2, true, "", 0},
	{"閃電资源", "閃電资源", "https://sdzyapi.com/api.php/provide/vod/", 2, true, "", 0},
	{"飘零资源", "飘零资源", "https://p2100.net/api.php/provide/vod/", 2, true, "优秀", 0},
	{"1080资源库", "1080资源库", "https://api.1080zyku.com/inc/api_mac10.php/", 2, true, "", 0},
	{"CK资源", "CK资源", "https://ckzy.me/api.php/provide/vod/", 2, true, "", 0},
	{"U酷资源", "U酷资源", "https://api.ukuapi.com/api.php/provide/vod/", 2, true, "", 0},
	{"丫丫点播", "丫丫点播", "https://cj.yayazy.net/api.php/provide/vod/", 2, true, "", 0},
	{"光速资源", "光速资源", "https://api.guangsuapi.com/api.php/provide/vod/", 2, true, "", 0},
	{"新浪点播", "新浪点播", "https://api.xinlangapi.com/xinlangapi.php/provide/vod/", 2, true, "优秀", 0},
	{"牛牛点播", "牛牛点播", "https://api.niuniuzy.me/api.php/provide/vod/", 2, true, "", 0},
	{"红牛资源", "红牛资源", "https://www.hongniuzy2.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"步步高资源", "步步高资源", "https://api.yparse.com/api/json", 2, true, "", 0},
	{"鸭鸭资源", "鸭鸭资源", "https://cj.yayazy.net/api.php/provide/vod/", 2, true, "", 0},
	{"影视工厂", "影视工厂", "https://cj.lziapi.com/api.php/provide/vod/", 2, true, "优秀", 0},
	{"快车资源", "快车资源", "https://caiji.kuaichezy.org/api.php/provide/vod/", 2, true, "", 0},
	{"极速资源", "极速资源", "https://jszyapi.com/api.php/provide/vod", 2, true, "", 0},
	{"卧龙资源", "卧龙资源", "https://wolongzyw.com/api.php/provide/vod", 2, true, "", 0},
	{"360", "360", "https://360zy.com/api.php/provide/vod", 2, true, "", 0},
}

// insertDefaultSources 插入默认视频源
func insertDefaultSources(db *sql.DB) {
	stmt, err := db.Prepare(`INSERT INTO video_sources (key, name, api, type, is_active, tags, priority, enabled, created_at) 
		VALUES (?, ?, ?, ?, ?, ?, ?, 1, datetime('now'))`)
	if err != nil {
		return
	}
	defer stmt.Close()

	for i, src := range defaultSources {
		isActive := 0
		if src.IsActive {
			isActive = 1
		}
		_, _ = stmt.Exec(src.Key, src.Name, src.API, src.Type, isActive, src.Tags, src.Priority, i)
	}
}
