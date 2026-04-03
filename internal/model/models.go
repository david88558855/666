package model

import "time"

// User 用户
type User struct {
	ID           int64     `json:"id"`
	Username     string    `json:"username"`
	PasswordHash string    `json:"-"`
	IsAdmin      bool      `json:"is_admin"`
	IsAdult      bool      `json:"is_adult"` // 成人内容过滤
	Theme        string    `json:"theme"`     // dark/light/auto
	Layout       string    `json:"layout"`     // grid/list
	ItemsPerPage int       `json:"items_per_page"`
	CreatedAt    time.Time `json:"created_at"`
	UpdatedAt    time.Time `json:"updated_at"`
}

// VideoSource 视频源
type VideoSource struct {
	ID        int64  `json:"id"`
	Name      string `json:"name"`
	API       string `json:"api"`
	DetailURL string `json:"detail"`
	IsAdult   bool   `json:"is_adult"`
	SortOrder int    `json:"sort_order"`
	Enabled   bool   `json:"enabled"`
	CreatedAt string `json:"created_at"`
}

// Favorite 收藏
type Favorite struct {
	ID        int64  `json:"id"`
	UserID    int64  `json:"user_id"`
	Site      string `json:"site"`
	SiteName  string `json:"site_name"`
	VideoID   string `json:"video_id"`
	Title     string `json:"title"`
	Cover     string `json:"cover"`
	Type      string `json:"type"` // movie/tv
	CreatedAt string `json:"created_at"`
}

// History 历史记录
type History struct {
	ID          int64  `json:"id"`
	UserID      int64  `json:"user_id"`
	Site        string `json:"site"`
	SiteName    string `json:"site_name"`
	VideoID     string `json:"video_id"`
	Title       string `json:"title"`
	Cover       string `json:"cover"`
	Type        string `json:"type"`
	Episode     int    `json:"episode"`
	Progress    int    `json:"progress"` // 秒
	Duration    int    `json:"duration"`
	UpdatedAt   string `json:"updated_at"`
}

// Config 系统配置
type Config struct {
	ID           int64  `json:"id"`
	Key          string `json:"key"`
	Value        string `json:"value"`
}

// AppConfig 应用配置 (来自 config.json)
type AppConfig struct {
	CacheTime int                   `json:"cache_time"`
	APISite   map[string]APISite   `json:"api_site"`
}

// APISite API站点
type APISite struct {
	API     string `json:"api"`
	Name    string `json:"name"`
	Detail  string `json:"detail,omitempty"`
	IsAdult bool   `json:"is_adult"`
}

// Cache 缓存
type Cache struct {
	ID        int64  `json:"id"`
	Key       string `json:"key"`
	Value     string `json:"value"`
	ExpiredAt int64  `json:"expired_at"`
}

// ========== 请求/响应结构 ==========

// RegisterRequest 注册请求
type RegisterRequest struct {
	Username string `json:"username" binding:"required,min=3,max=50"`
	Password string `json:"password" binding:"required,min=6,max=100"`
}

// LoginRequest 登录请求
type LoginRequest struct {
	Username string `json:"username" binding:"required"`
	Password string `json:"password" binding:"required"`
}

// LoginResponse 登录响应
type LoginResponse struct {
	Token string `json:"token"`
	User  *User  `json:"user"`
}

// AddSourceRequest 添加源请求
type AddSourceRequest struct {
	Name    string `json:"name" binding:"required"`
	API     string `json:"api" binding:"required"`
	Detail  string `json:"detail"`
	IsAdult bool   `json:"is_adult"`
}

// UpdateSourceRequest 更新源请求
type UpdateSourceRequest struct {
	Name    string `json:"name"`
	API     string `json:"api"`
	Detail  string `json:"detail"`
	IsAdult bool   `json:"is_adult"`
	Enabled *bool  `json:"enabled"`
}

// UpdateConfigRequest 更新配置请求
type UpdateConfigRequest struct {
	Config *AppConfig `json:"config"`
}

// ToggleRegisterRequest 切换注册请求
type ToggleRegisterRequest struct {
	Enabled bool `json:"enabled"`
}

// UpdateSettingsRequest 更新设置请求
type UpdateSettingsRequest struct {
	Theme        string `json:"theme"`
	Layout       string `json:"layout"`
	ItemsPerPage int    `json:"items_per_page"`
	IsAdult      bool   `json:"is_adult"`
}

// SearchResponse 搜索响应
type SearchResponse struct {
	Code    int         `json:"code"`
	Message string      `json:"message"`
	Data    []SearchItem `json:"data"`
}

// SearchItem 搜索结果项
type SearchItem struct {
	ID       string `json:"id"`
	Title    string `json:"title"`
	Cover    string `json:"cover"`
	Type     string `json:"type"`
	Site     string `json:"site"`
	SiteName string `json:"site_name"`
	Year     string `json:"year"`
	Note     string `json:"note"`
}

// DetailResponse 详情响应
type DetailResponse struct {
	Code    int         `json:"code"`
	Message string      `json:"message"`
	Data    *DetailData `json:"data"`
}

// DetailData 详情数据
type DetailData struct {
	ID          string      `json:"id"`
	Title       string      `json:"title"`
	Cover       string      `json:"cover"`
	Type        string      `json:"type"`
	Year        string      `json:"year"`
	Area        string      `json:"area"`
	Lang        string      `json:"lang"`
	Director    string      `json:"director"`
	Actor       string      `json:"actor"`
	Desc        string      `json:"desc"`
	Site        string      `json:"site"`
	SiteName    string      `json:"site_name"`
	Episodes    []Episode   `json:"episodes"`
}

// Episode 剧集
type Episode struct {
	EpisodeID string `json:"episode_id"`
	Name     string `json:"name"`
	PlayURL  string `json:"play_url,omitempty"`
}

// PlayResponse 播放响应
type PlayResponse struct {
	Code    int         `json:"code"`
	Message string      `json:"message"`
	Data    *PlayData   `json:"data"`
}

// PlayData 播放数据
type PlayData struct {
	URL      string `json:"url"`
	Headers  map[string]string `json:"headers,omitempty"`
}

// HomeResponse 首页数据响应
type HomeResponse struct {
	Code    int          `json:"code"`
	Message string       `json:"message"`
	Data    *HomeData    `json:"data"`
}

// HomeData 首页数据
type HomeData struct {
	Banner []BannerItem `json:"banner"`
	Hot    []SearchItem `json:"hot"`
	New    []SearchItem `json:"new"`
}

// BannerItem 轮播项
type BannerItem struct {
	Title string `json:"title"`
	Cover string `json:"cover"`
	Link  string `json:"link"`
	Type  string `json:"type"`
	ID    string `json:"id"`
}

// StatsResponse 统计响应
type StatsResponse struct {
	Code    int       `json:"code"`
	TotalUsers int    `json:"total_users"`
	TotalFavorites int `json:"total_favorites"`
	TotalHistory int  `json:"total_history"`
}

// APIResponse 通用响应
type APIResponse struct {
	Code    int         `json:"code"`
	Message string      `json:"message"`
	Data    interface{} `json:"data,omitempty"`
}

// ErrorResponse 错误响应
type ErrorResponse struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
}
