package repository

import (
	"database/sql"
	"time"

	"github.com/katelyatv/katelyatv-go/internal/model"
)

// UserRepository 用户仓库
type UserRepository struct {
	db *sql.DB
}

// NewUserRepository 创建用户仓库
func NewUserRepository(db *sql.DB) *UserRepository {
	return &UserRepository{db: db}
}

// Create 创建用户
func (r *UserRepository) Create(user *model.User) error {
	result, err := r.db.Exec(
		`INSERT INTO users (username, password_hash, is_admin, is_adult, theme, layout, items_per_page, created_at, updated_at) 
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		user.Username, user.PasswordHash, user.IsAdmin, user.IsAdult, user.Theme, user.Layout, user.ItemsPerPage, time.Now(), time.Now(),
	)
	if err != nil {
		return err
	}

	id, err := result.LastInsertId()
	if err != nil {
		return err
	}
	user.ID = id
	return nil
}

// GetByID 根据ID获取用户
func (r *UserRepository) GetByID(id int64) (*model.User, error) {
	user := &model.User{}
	err := r.db.QueryRow(
		`SELECT id, username, password_hash, is_admin, is_adult, theme, layout, items_per_page, created_at, updated_at 
		 FROM users WHERE id = ?`,
		id,
	).Scan(&user.ID, &user.Username, &user.PasswordHash, &user.IsAdmin, &user.IsAdult, &user.Theme, &user.Layout, &user.ItemsPerPage, &user.CreatedAt, &user.UpdatedAt)
	if err != nil {
		return nil, err
	}
	return user, nil
}

// GetByUsername 根据用户名获取用户
func (r *UserRepository) GetByUsername(username string) (*model.User, error) {
	user := &model.User{}
	err := r.db.QueryRow(
		`SELECT id, username, password_hash, is_admin, is_adult, theme, layout, items_per_page, created_at, updated_at 
		 FROM users WHERE username = ?`,
		username,
	).Scan(&user.ID, &user.Username, &user.PasswordHash, &user.IsAdmin, &user.IsAdult, &user.Theme, &user.Layout, &user.ItemsPerPage, &user.CreatedAt, &user.UpdatedAt)
	if err != nil {
		return nil, err
	}
	return user, nil
}

// GetAll 获取所有用户
func (r *UserRepository) GetAll() ([]*model.User, error) {
	rows, err := r.db.Query(
		`SELECT id, username, password_hash, is_admin, is_adult, theme, layout, items_per_page, created_at, updated_at 
		 FROM users ORDER BY created_at ASC`,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var users []*model.User
	for rows.Next() {
		user := &model.User{}
		if err := rows.Scan(&user.ID, &user.Username, &user.PasswordHash, &user.IsAdmin, &user.IsAdult, &user.Theme, &user.Layout, &user.ItemsPerPage, &user.CreatedAt, &user.UpdatedAt); err != nil {
			return nil, err
		}
		users = append(users, user)
	}
	return users, nil
}

// Count 统计用户数量
func (r *UserRepository) Count() (int, error) {
	var count int
	err := r.db.QueryRow("SELECT COUNT(*) FROM users").Scan(&count)
	return count, err
}

// Update 更新用户
func (r *UserRepository) Update(user *model.User) error {
	_, err := r.db.Exec(
		`UPDATE users SET is_adult = ?, theme = ?, layout = ?, items_per_page = ?, updated_at = ? WHERE id = ?`,
		user.IsAdult, user.Theme, user.Layout, user.ItemsPerPage, time.Now(), user.ID,
	)
	return err
}

// Delete 删除用户
func (r *UserRepository) Delete(id int64) error {
	_, err := r.db.Exec("DELETE FROM users WHERE id = ?", id)
	return err
}

// FavoriteRepository 收藏仓库
type FavoriteRepository struct {
	db *sql.DB
}

// NewFavoriteRepository 创建收藏仓库
func NewFavoriteRepository(db *sql.DB) *FavoriteRepository {
	return &FavoriteRepository{db: db}
}

// Create 创建收藏
func (r *FavoriteRepository) Create(fav *model.Favorite) error {
	result, err := r.db.Exec(
		`INSERT INTO favorites (user_id, site, site_name, video_id, title, cover, type, created_at) 
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
		fav.UserID, fav.Site, fav.SiteName, fav.VideoID, fav.Title, fav.Cover, fav.Type, time.Now(),
	)
	if err != nil {
		return err
	}
	id, _ := result.LastInsertId()
	fav.ID = id
	return nil
}

// GetByUserID 获取用户的所有收藏
func (r *FavoriteRepository) GetByUserID(userID int64) ([]*model.Favorite, error) {
	rows, err := r.db.Query(
		`SELECT id, user_id, site, site_name, video_id, title, cover, type, created_at 
		 FROM favorites WHERE user_id = ? ORDER BY created_at DESC`,
		userID,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var favorites []*model.Favorite
	for rows.Next() {
		fav := &model.Favorite{}
		if err := rows.Scan(&fav.ID, &fav.UserID, &fav.Site, &fav.SiteName, &fav.VideoID, &fav.Title, &fav.Cover, &fav.Type, &fav.CreatedAt); err != nil {
			return nil, err
		}
		favorites = append(favorites, fav)
	}
	return favorites, nil
}

// GetByIDAndUserID 根据ID和用户ID获取收藏
func (r *FavoriteRepository) GetByIDAndUserID(id, userID int64) (*model.Favorite, error) {
	fav := &model.Favorite{}
	err := r.db.QueryRow(
		`SELECT id, user_id, site, site_name, video_id, title, cover, type, created_at 
		 FROM favorites WHERE id = ? AND user_id = ?`,
		id, userID,
	).Scan(&fav.ID, &fav.UserID, &fav.Site, &fav.SiteName, &fav.VideoID, &fav.Title, &fav.Cover, &fav.Type, &fav.CreatedAt)
	if err != nil {
		return nil, err
	}
	return fav, nil
}

// Delete 删除收藏
func (r *FavoriteRepository) Delete(id, userID int64) error {
	_, err := r.db.Exec("DELETE FROM favorites WHERE id = ? AND user_id = ?", id, userID)
	return err
}

// Exists 检查收藏是否存在
func (r *FavoriteRepository) Exists(userID int64, site, videoID string) (bool, error) {
	var count int
	err := r.db.QueryRow(
		"SELECT COUNT(*) FROM favorites WHERE user_id = ? AND site = ? AND video_id = ?",
		userID, site, videoID,
	).Scan(&count)
	return count > 0, err
}

// Count 统计收藏数量
func (r *FavoriteRepository) Count() (int, error) {
	var count int
	err := r.db.QueryRow("SELECT COUNT(*) FROM favorites").Scan(&count)
	return count, err
}

// HistoryRepository 历史记录仓库
type HistoryRepository struct {
	db *sql.DB
}

// NewHistoryRepository 创建历史记录仓库
func NewHistoryRepository(db *sql.DB) *HistoryRepository {
	return &HistoryRepository{db: db}
}

// Upsert 创建或更新历史记录
func (r *HistoryRepository) Upsert(h *model.History) error {
	_, err := r.db.Exec(
		`INSERT INTO history (user_id, site, site_name, video_id, title, cover, type, episode, progress, duration, updated_at) 
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		 ON CONFLICT(user_id, site, video_id, episode) DO UPDATE SET
		 progress = excluded.progress, duration = excluded.duration, updated_at = excluded.updated_at`,
		h.UserID, h.Site, h.SiteName, h.VideoID, h.Title, h.Cover, h.Type, h.Episode, h.Progress, h.Duration, time.Now(),
	)
	return err
}

// GetByUserID 获取用户的所有历史记录
func (r *HistoryRepository) GetByUserID(userID int64, limit int) ([]*model.History, error) {
	rows, err := r.db.Query(
		`SELECT id, user_id, site, site_name, video_id, title, cover, type, episode, progress, duration, updated_at 
		 FROM history WHERE user_id = ? ORDER BY updated_at DESC LIMIT ?`,
		userID, limit,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var histories []*model.History
	for rows.Next() {
		h := &model.History{}
		if err := rows.Scan(&h.ID, &h.UserID, &h.Site, &h.SiteName, &h.VideoID, &h.Title, &h.Cover, &h.Type, &h.Episode, &h.Progress, &h.Duration, &h.UpdatedAt); err != nil {
			return nil, err
		}
		histories = append(histories, h)
	}
	return histories, nil
}

// Delete 删除历史记录
func (r *HistoryRepository) Delete(id, userID int64) error {
	_, err := r.db.Exec("DELETE FROM history WHERE id = ? AND user_id = ?", id, userID)
	return err
}

// Clear 清空用户历史
func (r *HistoryRepository) Clear(userID int64) error {
	_, err := r.db.Exec("DELETE FROM history WHERE user_id = ?", userID)
	return err
}

// Count 统计历史记录数量
func (r *HistoryRepository) Count() (int, error) {
	var count int
	err := r.db.QueryRow("SELECT COUNT(*) FROM history").Scan(&count)
	return count, err
}
