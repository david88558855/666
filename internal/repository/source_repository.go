package repository

import (
	"database/sql"
	"time"

	"github.com/katelyatv/katelyatv-go/internal/model"
)

// SourceRepository 视频源仓库
type SourceRepository struct {
	db *sql.DB
}

// NewSourceRepository 创建视频源仓库
func NewSourceRepository(db *sql.DB) *SourceRepository {
	return &SourceRepository{db: db}
}

// Create 创建视频源
func (r *SourceRepository) Create(source *model.VideoSource) error {
	result, err := r.db.Exec(
		`INSERT INTO video_sources (name, api, detail, is_adult, sort_order, enabled, created_at) 
		 VALUES (?, ?, ?, ?, ?, ?, ?)`,
		source.Name, source.API, source.Detail, source.IsAdult, source.SortOrder, source.Enabled, time.Now(),
	)
	if err != nil {
		return err
	}
	id, _ := result.LastInsertId()
	source.ID = id
	return nil
}

// GetByID 根据ID获取视频源
func (r *SourceRepository) GetByID(id int64) (*model.VideoSource, error) {
	source := &model.VideoSource{}
	err := r.db.QueryRow(
		`SELECT id, name, api, detail, is_adult, sort_order, enabled, created_at 
		 FROM video_sources WHERE id = ?`,
		id,
	).Scan(&source.ID, &source.Name, &source.API, &source.Detail, &source.IsAdult, &source.SortOrder, &source.Enabled, &source.CreatedAt)
	if err != nil {
		return nil, err
	}
	return source, nil
}

// GetAll 获取所有视频源
func (r *SourceRepository) GetAll() ([]*model.VideoSource, error) {
	rows, err := r.db.Query(
		`SELECT id, name, api, detail, is_adult, sort_order, enabled, created_at 
		 FROM video_sources ORDER BY sort_order ASC, id ASC`,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var sources []*model.VideoSource
	for rows.Next() {
		source := &model.VideoSource{}
		if err := rows.Scan(&source.ID, &source.Name, &source.API, &source.Detail, &source.IsAdult, &source.SortOrder, &source.Enabled, &source.CreatedAt); err != nil {
			return nil, err
		}
		sources = append(sources, source)
	}
	return sources, nil
}

// GetEnabled 获取所有启用的视频源
func (r *SourceRepository) GetEnabled() ([]*model.VideoSource, error) {
	rows, err := r.db.Query(
		`SELECT id, name, api, detail, is_adult, sort_order, enabled, created_at 
		 FROM video_sources WHERE enabled = 1 ORDER BY sort_order ASC, id ASC`,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var sources []*model.VideoSource
	for rows.Next() {
		source := &model.VideoSource{}
		if err := rows.Scan(&source.ID, &source.Name, &source.API, &source.Detail, &source.IsAdult, &source.SortOrder, &source.Enabled, &source.CreatedAt); err != nil {
			return nil, err
		}
		sources = append(sources, source)
	}
	return sources, nil
}

// GetEnabledNoAdult 获取所有启用且非成人的视频源
func (r *SourceRepository) GetEnabledNoAdult() ([]*model.VideoSource, error) {
	rows, err := r.db.Query(
		`SELECT id, name, api, detail, is_adult, sort_order, enabled, created_at 
		 FROM video_sources WHERE enabled = 1 AND is_adult = 0 ORDER BY sort_order ASC, id ASC`,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var sources []*model.VideoSource
	for rows.Next() {
		source := &model.VideoSource{}
		if err := rows.Scan(&source.ID, &source.Name, &source.API, &source.Detail, &source.IsAdult, &source.SortOrder, &source.Enabled, &source.CreatedAt); err != nil {
			return nil, err
		}
		sources = append(sources, source)
	}
	return sources, nil
}

// Update 更新视频源
func (r *SourceRepository) Update(source *model.VideoSource) error {
	_, err := r.db.Exec(
		`UPDATE video_sources SET name = ?, api = ?, detail = ?, is_adult = ?, sort_order = ?, enabled = ? WHERE id = ?`,
		source.Name, source.API, source.Detail, source.IsAdult, source.SortOrder, source.Enabled, source.ID,
	)
	return err
}

// Delete 删除视频源
func (r *SourceRepository) Delete(id int64) error {
	_, err := r.db.Exec("DELETE FROM video_sources WHERE id = ?", id)
	return err
}

// Count 统计视频源数量
func (r *SourceRepository) Count() (int, error) {
	var count int
	err := r.db.QueryRow("SELECT COUNT(*) FROM video_sources").Scan(&count)
	return count, err
}

// SettingRepository 设置仓库
type SettingRepository struct {
	db *sql.DB
}

// NewSettingRepository 创建设置仓库
func NewSettingRepository(db *sql.DB) *SettingRepository {
	return &SettingRepository{db: db}
}

// Get 获取设置值
func (r *SettingRepository) Get(key string) (string, error) {
	var value string
	err := r.db.QueryRow("SELECT value FROM settings WHERE key = ?", key).Scan(&value)
	if err != nil {
		return "", err
	}
	return value, nil
}

// Set 设置值
func (r *SettingRepository) Set(key, value string) error {
	_, err := r.db.Exec(
		`INSERT INTO settings (key, value) VALUES (?, ?) 
		 ON CONFLICT(key) DO UPDATE SET value = excluded.value`,
		key, value,
	)
	return err
}

// GetAll 获取所有设置
func (r *SettingRepository) GetAll() (map[string]string, error) {
	rows, err := r.db.Query("SELECT key, value FROM settings")
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	settings := make(map[string]string)
	for rows.Next() {
		var key, value string
		if err := rows.Scan(&key, &value); err != nil {
			return nil, err
		}
		settings[key] = value
	}
	return settings, nil
}

// CacheRepository 缓存仓库
type CacheRepository struct {
	db *sql.DB
}

// NewCacheRepository 创建缓存仓库
func NewCacheRepository(db *sql.DB) *CacheRepository {
	return &CacheRepository{db: db}
}

// Get 获取缓存
func (r *CacheRepository) Get(key string) (string, error) {
	var value string
	err := r.db.QueryRow(
		`SELECT value FROM cache WHERE key = ? AND expired_at > ?`,
		key, time.Now().Unix(),
	).Scan(&value)
	if err != nil {
		return "", err
	}
	return value, nil
}

// Set 设置缓存
func (r *CacheRepository) Set(key, value string, expireSeconds int) error {
	expiredAt := time.Now().Unix() + int64(expireSeconds)
	_, err := r.db.Exec(
		`INSERT INTO cache (key, value, expired_at) VALUES (?, ?, ?) 
		 ON CONFLICT(key) DO UPDATE SET value = excluded.value, expired_at = excluded.expired_at`,
		key, value, expiredAt,
	)
	return err
}

// Delete 删除缓存
func (r *CacheRepository) Delete(key string) error {
	_, err := r.db.Exec("DELETE FROM cache WHERE key = ?", key)
	return err
}

// CleanExpired 清理过期缓存
func (r *CacheRepository) CleanExpired() error {
	_, err := r.db.Exec("DELETE FROM cache WHERE expired_at <= ?", time.Now().Unix())
	return err
}
