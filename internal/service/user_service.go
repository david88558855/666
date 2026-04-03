package service

import (
	"database/sql"
	"errors"

	"github.com/katelyatv/katelyatv-go/internal/model"
	"github.com/katelyatv/katelyatv-go/internal/repository"

	"golang.org/x/crypto/bcrypt"
)

var (
	ErrUserExists        = errors.New("用户名已存在")
	ErrUserNotFound      = errors.New("用户不存在")
	ErrInvalidPassword   = errors.New("密码错误")
	ErrRegisterClosed    = errors.New("注册已关闭")
	ErrCannotDeleteAdmin = errors.New("不能删除管理员用户")
)

// UserService 用户服务
type UserService struct {
	repo         *repository.UserRepository
	favRepo      *repository.FavoriteRepository
	historyRepo  *repository.HistoryRepository
	registerOpen bool
}

// NewUserService 创建用户服务
func NewUserService(repo *repository.UserRepository, favRepo *repository.FavoriteRepository, historyRepo *repository.HistoryRepository) *UserService {
	svc := &UserService{
		repo:         repo,
		favRepo:      favRepo,
		historyRepo:  historyRepo,
		registerOpen: true,
	}
	// 检查是否已有用户
	count, _ := repo.Count()
	svc.registerOpen = count == 0 // 第一个用户注册前开放注册
	return svc
}

// IsRegisterOpen 检查注册是否开放
func (s *UserService) IsRegisterOpen() bool {
	return s.registerOpen
}

// SetRegisterOpen 设置注册开关
func (s *UserService) SetRegisterOpen(open bool) {
	s.registerOpen = open
}

// Register 注册
func (s *UserService) Register(req *model.RegisterRequest) (*model.User, error) {
	if !s.registerOpen {
		return nil, ErrRegisterClosed
	}

	// 检查用户名是否存在
	_, err := s.repo.GetByUsername(req.Username)
	if err == nil {
		return nil, ErrUserExists
	}

	// 哈希密码
	hash, err := bcrypt.GenerateFromPassword([]byte(req.Password), bcrypt.DefaultCost)
	if err != nil {
		return nil, err
	}

	// 检查是否是第一个用户
	count, _ := s.repo.Count()
	isAdmin := count == 0

	user := &model.User{
		Username:     req.Username,
		PasswordHash: string(hash),
		IsAdmin:      isAdmin,
		IsAdult:      false,
		Theme:        "auto",
		Layout:       "grid",
		ItemsPerPage: 24,
	}

	if err := s.repo.Create(user); err != nil {
		return nil, err
	}

	// 如果是第一个用户，注册后关闭注册
	if isAdmin {
		s.registerOpen = false
	}

	return user, nil
}

// Login 登录
func (s *UserService) Login(req *model.LoginRequest) (*model.User, error) {
	user, err := s.repo.GetByUsername(req.Username)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, ErrUserNotFound
		}
		return nil, err
	}

	if err := bcrypt.CompareHashAndPassword([]byte(user.PasswordHash), []byte(req.Password)); err != nil {
		return nil, ErrInvalidPassword
	}

	return user, nil
}

// GetByID 获取用户
func (s *UserService) GetByID(id int64) (*model.User, error) {
	return s.repo.GetByID(id)
}

// GetAll 获取所有用户
func (s *UserService) GetAll() ([]*model.User, error) {
	return s.repo.GetAll()
}

// Update 更新用户
func (s *UserService) Update(user *model.User) error {
	return s.repo.Update(user)
}

// Delete 删除用户
func (s *UserService) Delete(id int64, currentUserID int64) error {
	user, err := s.repo.GetByID(id)
	if err != nil {
		return err
	}

	if user.IsAdmin {
		return ErrCannotDeleteAdmin
	}

	return s.repo.Delete(id)
}

// CreateUserByAdmin 管理员创建用户
func (s *UserService) CreateUserByAdmin(username, password string, isAdmin bool) (*model.User, error) {
	// 检查用户名是否存在
	_, err := s.repo.GetByUsername(username)
	if err == nil {
		return nil, ErrUserExists
	}
	if err != sql.ErrNoRows {
		return nil, err
	}

	// 哈希密码
	hash, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		return nil, err
	}

	user := &model.User{
		Username:     username,
		PasswordHash: string(hash),
		IsAdmin:      isAdmin,
		IsAdult:      false,
		Theme:        "auto",
		Layout:       "grid",
		ItemsPerPage: 24,
	}

	if err := s.repo.Create(user); err != nil {
		return nil, err
	}

	return user, nil
}

// GetFavorites 获取收藏
func (s *UserService) GetFavorites(userID int64) ([]*model.Favorite, error) {
	return s.favRepo.GetByUserID(userID)
}

// AddFavorite 添加收藏
func (s *UserService) AddFavorite(userID int64, site, siteName, videoID, title, cover, videoType string) (*model.Favorite, error) {
	fav := &model.Favorite{
		UserID:   userID,
		Site:     site,
		SiteName: siteName,
		VideoID:  videoID,
		Title:    title,
		Cover:    cover,
		Type:     videoType,
	}

	if err := s.favRepo.Create(fav); err != nil {
		return nil, err
	}

	return fav, nil
}

// RemoveFavorite 删除收藏
func (s *UserService) RemoveFavorite(id, userID int64) error {
	return s.favRepo.Delete(id, userID)
}

// FavoriteExists 检查收藏是否存在
func (s *UserService) FavoriteExists(userID int64, site, videoID string) (bool, error) {
	return s.favRepo.Exists(userID, site, videoID)
}

// GetHistory 获取历史记录
func (s *UserService) GetHistory(userID int64, limit int) ([]*model.History, error) {
	if limit <= 0 {
		limit = 100
	}
	return s.historyRepo.GetByUserID(userID, limit)
}

// AddHistory 添加历史记录
func (s *UserService) AddHistory(userID int64, site, siteName, videoID, title, cover, videoType string, episode, progress, duration int) error {
	h := &model.History{
		UserID:   userID,
		Site:     site,
		SiteName: siteName,
		VideoID:  videoID,
		Title:    title,
		Cover:    cover,
		Type:     videoType,
		Episode:  episode,
		Progress: progress,
		Duration: duration,
	}
	return s.historyRepo.Upsert(h)
}

// ClearHistory 清空历史
func (s *UserService) ClearHistory(userID int64) error {
	return s.historyRepo.Clear(userID)
}

// CountUsers 统计用户数
func (s *UserService) CountUsers() (int, error) {
	return s.repo.Count()
}

// CountFavorites 统计收藏数
func (s *UserService) CountFavorites() (int, error) {
	return s.favRepo.Count()
}

// CountHistory 统计历史数
func (s *UserService) CountHistory() (int, error) {
	return s.historyRepo.Count()
}

// UserServiceWithRepos 带仓库的服务
type UserServiceWithRepos struct {
	*UserService
	favRepo     *repository.FavoriteRepository
	historyRepo *repository.HistoryRepository
}

// NewUserServiceWithRepos 创建带仓库的用户服务
func NewUserServiceWithRepos(
	userRepo *repository.UserRepository,
	favRepo *repository.FavoriteRepository,
	historyRepo *repository.HistoryRepository,
	registerOpen bool,
) *UserServiceWithRepos {
	return &UserServiceWithRepos{
		UserService: &UserService{repo: userRepo, registerOpen: registerOpen},
		favRepo:     favRepo,
		historyRepo: historyRepo,
	}
}

// UserServiceInterface 用户服务接口
type UserServiceInterface interface {
	Register(req *model.RegisterRequest) (*model.User, error)
	Login(req *model.LoginRequest) (*model.User, error)
	GetByID(id int64) (*model.User, error)
	Update(user *model.User) error
	Delete(id int64, currentUserID int64) error
	IsRegisterOpen() bool
	SetRegisterOpen(open bool)
}

// SimpleUserService 简化用户服务（用于初始化）
type SimpleUserService struct {
	repo *repository.UserRepository
}

func NewSimpleUserService(repo *repository.UserRepository) *SimpleUserService {
	return &SimpleUserService{repo: repo}
}
