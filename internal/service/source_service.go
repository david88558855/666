package service

import (
	"database/sql"

	"github.com/katelyatv/katelyatv-go/internal/model"
	"github.com/katelyatv/katelyatv-go/internal/repository"
)

var (
	ErrSourceNotFound = sql.ErrNoRows
)

// SourceService 视频源服务
type SourceService struct {
	repo *repository.SourceRepository
}

// NewSourceService 创建视频源服务
func NewSourceService(repo *repository.SourceRepository) *SourceService {
	return &SourceService{repo: repo}
}

// GetAll 获取所有视频源
func (s *SourceService) GetAll() ([]*model.VideoSource, error) {
	return s.repo.GetAll()
}

// GetEnabled 获取所有启用的视频源
func (s *SourceService) GetEnabled() ([]*model.VideoSource, error) {
	return s.repo.GetEnabled()
}

// GetEnabledNoAdult 获取所有启用且非成人的视频源
func (s *SourceService) GetEnabledNoAdult() ([]*model.VideoSource, error) {
	return s.repo.GetEnabledNoAdult()
}

// GetByID 根据ID获取视频源
func (s *SourceService) GetByID(id int64) (*model.VideoSource, error) {
	return s.repo.GetByID(id)
}

// Create 创建视频源
func (s *SourceService) Create(source *model.VideoSource) error {
	return s.repo.Create(source)
}

// Update 更新视频源
func (s *SourceService) Update(source *model.VideoSource) error {
	return s.repo.Update(source)
}

// Delete 删除视频源
func (s *SourceService) Delete(id int64) error {
	return s.repo.Delete(id)
}

// Count 统计数量
func (s *SourceService) Count() (int, error) {
	return s.repo.Count()
}

// SettingService 设置服务
type SettingService struct {
	repo        *repository.SettingRepository
	configPath  string
	config      *model.AppConfig
}

// NewSettingService 创建设置服务
func NewSettingService(repo *repository.SettingRepository, configPath string) *SettingService {
	s := &SettingService{
		repo:       repo,
		configPath: configPath,
		config:     &model.AppConfig{},
	}
	s.loadConfig()
	return s
}

// loadConfig 加载配置
func (s *SettingService) loadConfig() {
	// 优先从 config.json 加载
	if s.configPath != "" {
		s.config = LoadAppConfig(s.configPath)
	}

	// 从数据库加载注册设置
	if enabled, err := s.repo.Get("register_enabled"); err == nil {
		// 已设置
	}
}

// GetConfig 获取配置
func (s *SettingService) GetConfig() *model.AppConfig {
	return s.config
}

// UpdateConfig 更新配置
func (s *SettingService) UpdateConfig(config *model.AppConfig) error {
	s.config = config
	SaveAppConfig(s.configPath, config)
	return nil
}

// GetRegisterEnabled 获取注册是否启用
func (s *SettingService) GetRegisterEnabled() bool {
	val, err := s.repo.Get("register_enabled")
	if err != nil {
		return true // 默认开启
	}
	return val == "true"
}

// SetRegisterEnabled 设置注册是否启用
func (s *SettingService) SetRegisterEnabled(enabled bool) error {
	return s.repo.Set("register_enabled", map[bool]string{true: "true", false: "false"}[enabled])
}

// GetCacheTime 获取缓存时间
func (s *SettingService) GetCacheTime() int {
	if s.config != nil && s.config.CacheTime > 0 {
		return s.config.CacheTime
	}
	return 7200 // 默认2小时
}

// CacheService 缓存服务
type CacheService struct {
	repo *repository.CacheRepository
}

// NewCacheService 创建缓存服务
func NewCacheService(repo *repository.CacheRepository) *CacheService {
	return &CacheService{repo: repo}
}

// Get 获取缓存
func (s *CacheService) Get(key string) (string, error) {
	return s.repo.Get(key)
}

// Set 设置缓存
func (s *CacheService) Set(key, value string, expireSeconds int) error {
	return s.repo.Set(key, value, expireSeconds)
}

// Delete 删除缓存
func (s *CacheService) Delete(key string) error {
	return s.repo.Delete(key)
}

// CleanExpired 清理过期缓存
func (s *CacheService) CleanExpired() error {
	return s.repo.CleanExpired()
}
