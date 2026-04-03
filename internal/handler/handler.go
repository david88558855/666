package handler

import (
	"fmt"
	"net/http"
	"net/url"
	"strconv"
	"strings"

	"github.com/gin-gonic/gin"
	"github.com/katelyatv/katelyatv-go/internal/model"
	"github.com/katelyatv/katelyatv-go/internal/service"
)

// Handler 处理器
type Handler struct {
	userService    *service.UserService
	sourceService  *service.SourceService
	settingService *service.SettingService
	searchService  *service.SearchService
}

// NewHandler 创建处理器
func NewHandler(
	userService *service.UserService,
	sourceService *service.SourceService,
	settingService *service.SettingService,
	searchService *service.SearchService,
) *Handler {
	return &Handler{
		userService:    userService,
		sourceService:  sourceService,
		settingService: settingService,
		searchService:  searchService,
	}
}

// ========== 中间件 ==========

// AuthRequired 认证中间件
func (h *Handler) AuthRequired() gin.HandlerFunc {
	return func(c *gin.Context) {
		token := c.GetHeader("Authorization")
		if token == "" {
			token = c.Query("token")
		}
		token = strings.TrimPrefix(token, "Bearer ")

		if token == "" {
			c.JSON(http.StatusUnauthorized, model.APIResponse{
				Code:    401,
				Message: "请先登录",
			})
			c.Abort()
			return
		}

		// 简单token验证（实际应使用JWT）
		userID, err := strconv.ParseInt(token, 10, 64)
		if err != nil {
			c.JSON(http.StatusUnauthorized, model.APIResponse{
				Code:    401,
				Message: "无效的token",
			})
			c.Abort()
			return
		}

		user, err := h.userService.GetByID(userID)
		if err != nil {
			c.JSON(http.StatusUnauthorized, model.APIResponse{
				Code:    401,
				Message: "用户不存在",
			})
			c.Abort()
			return
		}

		c.Set("user", user)
		c.Next()
	}
}

// AdminRequired 管理员中间件
func (h *Handler) AdminRequired() gin.HandlerFunc {
	return func(c *gin.Context) {
		user, exists := c.Get("user")
		if !exists {
			c.JSON(http.StatusUnauthorized, model.APIResponse{
				Code:    401,
				Message: "请先登录",
			})
			c.Abort()
			return
		}

		u := user.(*model.User)
		if !u.IsAdmin {
			c.JSON(http.StatusForbidden, model.APIResponse{
				Code:    403,
				Message: "需要管理员权限",
			})
			c.Abort()
			return
		}

		c.Next()
	}
}

// ========== 页面路由 ==========

// Index 首页
func (h *Handler) Index(c *gin.Context) {
	c.HTML(http.StatusOK, "index.html", gin.H{
		"Title": "KatelyaTV-Go",
	})
}

// Play 播放页
func (h *Handler) Play(c *gin.Context) {
	c.HTML(http.StatusOK, "play.html", gin.H{
		"Title": "播放",
	})
}

// SearchPage 搜索页
func (h *Handler) SearchPage(c *gin.Context) {
	c.HTML(http.StatusOK, "search.html", gin.H{
		"Title": "搜索",
	})
}

// AdminPage 管理页
func (h *Handler) AdminPage(c *gin.Context) {
	c.HTML(http.StatusOK, "admin.html", gin.H{
		"Title": "管理后台",
	})
}

// LoginPage 登录页
func (h *Handler) LoginPage(c *gin.Context) {
	c.HTML(http.StatusOK, "login.html", gin.H{
		"Title": "登录",
	})
}

// RegisterPage 注册页
func (h *Handler) RegisterPage(c *gin.Context) {
	c.HTML(http.StatusOK, "register.html", gin.H{
		"Title":   "注册",
		"Enabled": h.userService.IsRegisterOpen(),
	})
}

// SettingsPage 设置页
func (h *Handler) SettingsPage(c *gin.Context) {
	c.HTML(http.StatusOK, "settings.html", gin.H{
		"Title": "设置",
	})
}

// ========== 认证API ==========

// Register 注册
func (h *Handler) Register(c *gin.Context) {
	var req model.RegisterRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误: " + err.Error(),
		})
		return
	}

	user, err := h.userService.Register(&req)
	if err != nil {
		code := 500
		if err == service.ErrUserExists {
			code = 409
		} else if err == service.ErrRegisterClosed {
			code = 403
		}
		c.JSON(code, model.APIResponse{
			Code:    code,
			Message: err.Error(),
		})
		return
	}

	// 生成简单token
	token := fmt.Sprintf("%d", user.ID)

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "注册成功",
		Data: map[string]interface{}{
			"token": token,
			"user":  user,
		},
	})
}

// Login 登录
func (h *Handler) Login(c *gin.Context) {
	var req model.LoginRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误: " + err.Error(),
		})
		return
	}

	user, err := h.userService.Login(&req)
	if err != nil {
		code := 500
		if err == service.ErrUserNotFound || err == service.ErrInvalidPassword {
			code = 401
		}
		c.JSON(code, model.APIResponse{
			Code:    code,
			Message: err.Error(),
		})
		return
	}

	// 生成简单token
	token := fmt.Sprintf("%d", user.ID)

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "登录成功",
		Data: map[string]interface{}{
			"token": token,
			"user":  user,
		},
	})
}

// Logout 登出
func (h *Handler) Logout(c *gin.Context) {
	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "已退出登录",
	})
}

// ========== 公开API ==========

// GetCategories 获取分类
func (h *Handler) GetCategories(c *gin.Context) {
	categories := h.searchService.GetCategories()
	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "success",
		Data:    categories,
	})
}

// Search 搜索
func (h *Handler) Search(c *gin.Context) {
	query := c.Query("q")
	if query == "" {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "请输入搜索关键词",
		})
		return
	}

	// 检查用户是否登录，过滤成人内容
	filterAdult := true
	if user, exists := c.Get("user"); exists {
		u := user.(*model.User)
		filterAdult = u.IsAdult
	}

	results, err := h.searchService.Search(query, filterAdult)
	if err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "搜索失败: " + err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, model.SearchResponse{
		Code:    0,
		Message: "success",
		Data:    results,
	})
}

// GetDetail 获取详情
func (h *Handler) GetDetail(c *gin.Context) {
	site := c.Param("site")
	id := c.Param("id")

	// URL解码
	site, _ = url.QueryUnescape(site)

	detail, err := h.searchService.GetDetail(site, id)
	if err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "获取详情失败: " + err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, model.DetailResponse{
		Code:    0,
		Message: "success",
		Data:    detail,
	})
}

// GetPlayUrl 获取播放地址
func (h *Handler) GetPlayUrl(c *gin.Context) {
	site := c.Param("site")
	id := c.Param("id")
	episodeID := c.Query("episode")

	// URL解码
	site, _ = url.QueryUnescape(site)

	detail, err := h.searchService.GetDetail(site, id)
	if err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "获取详情失败: " + err.Error(),
		})
		return
	}

	// 找到对应剧集
	var playURL string
	for _, ep := range detail.Episodes {
		if ep.EpisodeID == episodeID {
			playURL = ep.PlayURL
			break
		}
	}

	if playURL == "" && len(detail.Episodes) > 0 {
		playURL = detail.Episodes[0].PlayURL
	}

	c.JSON(http.StatusOK, model.PlayResponse{
		Code:    0,
		Message: "success",
		Data: &model.PlayData{
			URL: playURL,
		},
	})
}

// GetHomeData 获取首页数据
func (h *Handler) GetHomeData(c *gin.Context) {
	data, err := h.searchService.GetHomeData()
	if err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "获取数据失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.HomeResponse{
		Code:    0,
		Message: "success",
		Data:    data,
	})
}

// GetTVBoxConfig 获取TVBox配置
func (h *Handler) GetTVBoxConfig(c *gin.Context) {
	format := c.DefaultQuery("format", "json")

	sources, err := h.sourceService.GetEnabled()
	if err != nil {
		sources = []*model.VideoSource{}
	}

	if format == "txt" {
		// TXT格式
		var lines []string
		for _, s := range sources {
			lines = append(lines, fmt.Sprintf("%s=%s", s.Name, s.API))
		}
		c.Header("Content-Type", "text/plain; charset=utf-8")
		c.String(http.StatusOK, strings.Join(lines, "\n"))
		return
	}

	// JSON格式
	type TVBoxSource struct {
		Name string `json:"name"`
		URL  string `json:"url"`
		Type string `json:"type"`
		Flag string `json:"flag"`
	}

	var tvSources []TVBoxSource
	for _, s := range sources {
		tvSources = append(tvSources, TVBoxSource{
			Name: s.Name,
			URL:  s.API,
			Type: "dmc",
			Flag: "drpy",
		})
	}

	c.JSON(http.StatusOK, gin.H{
		"api":     "/api/tvbox",
		"name":    "KatelyaTV-Go",
		"version": "1.0.0",
		"lives":   []interface{}{},
		"flags":   []string{"drpy", "drpy2", "cms"},
		"spider":  "/spider/latest.jar",
		"sites":   tvSources,
	})
}

// ========== 用户API ==========

// GetUserInfo 获取用户信息
func (h *Handler) GetUserInfo(c *gin.Context) {
	user, _ := c.Get("user")
	u := user.(*model.User)
	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "success",
		Data:    u,
	})
}

// GetFavorites 获取收藏
func (h *Handler) GetFavorites(c *gin.Context) {
	user, _ := c.Get("user")
	u := user.(*model.User)

	favorites, err := h.userService.GetFavorites(u.ID)
	if err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "获取收藏失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "success",
		Data:    favorites,
	})
}

// AddFavorite 添加收藏
func (h *Handler) AddFavorite(c *gin.Context) {
	user, _ := c.Get("user")
	u := user.(*model.User)

	var req struct {
		Site     string `json:"site" binding:"required"`
		SiteName string `json:"site_name" binding:"required"`
		VideoID  string `json:"video_id" binding:"required"`
		Title    string `json:"title" binding:"required"`
		Cover    string `json:"cover"`
		Type     string `json:"type"`
	}

	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误",
		})
		return
	}

	if req.Type == "" {
		req.Type = "movie"
	}

	fav, err := h.userService.AddFavorite(u.ID, req.Site, req.SiteName, req.VideoID, req.Title, req.Cover, req.Type)
	if err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "添加收藏失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "添加成功",
		Data:    fav,
	})
}

// RemoveFavorite 删除收藏
func (h *Handler) RemoveFavorite(c *gin.Context) {
	user, _ := c.Get("user")
	u := user.(*model.User)

	id, _ := strconv.ParseInt(c.Param("id"), 10, 64)

	if err := h.userService.RemoveFavorite(id, u.ID); err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "删除失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "删除成功",
	})
}

// GetHistory 获取历史
func (h *Handler) GetHistory(c *gin.Context) {
	user, _ := c.Get("user")
	u := user.(*model.User)

	history, err := h.userService.GetHistory(u.ID, 100)
	if err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "获取历史失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "success",
		Data:    history,
	})
}

// AddHistory 添加历史
func (h *Handler) AddHistory(c *gin.Context) {
	user, _ := c.Get("user")
	u := user.(*model.User)

	var req struct {
		Site     string `json:"site" binding:"required"`
		SiteName string `json:"site_name"`
		VideoID  string `json:"video_id" binding:"required"`
		Title    string `json:"title" binding:"required"`
		Cover    string `json:"cover"`
		Type     string `json:"type"`
		Episode  int    `json:"episode"`
		Progress int    `json:"progress"`
		Duration int    `json:"duration"`
	}

	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误",
		})
		return
	}

	if req.Type == "" {
		req.Type = "movie"
	}

	if err := h.userService.AddHistory(u.ID, req.Site, req.SiteName, req.VideoID, req.Title, req.Cover, req.Type, req.Episode, req.Progress, req.Duration); err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "添加历史失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "success",
	})
}

// UpdateUserSettings 更新用户设置
func (h *Handler) UpdateUserSettings(c *gin.Context) {
	user, _ := c.Get("user")
	u := user.(*model.User)

	var req model.UpdateSettingsRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误",
		})
		return
	}

	if req.Theme != "" {
		u.Theme = req.Theme
	}
	if req.Layout != "" {
		u.Layout = req.Layout
	}
	if req.ItemsPerPage > 0 {
		u.ItemsPerPage = req.ItemsPerPage
	}
	u.IsAdult = req.IsAdult

	if err := h.userService.Update(u); err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "更新失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "更新成功",
		Data:    u,
	})
}

// ========== 管理API ==========

// GetUsers 获取用户列表
func (h *Handler) GetUsers(c *gin.Context) {
	users, err := h.userService.GetAll()
	if err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "获取用户列表失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "success",
		Data:    users,
	})
}

// DeleteUser 删除用户
func (h *Handler) DeleteUser(c *gin.Context) {
	currentUser, _ := c.Get("user")
	cu := currentUser.(*model.User)

	id, _ := strconv.ParseInt(c.Param("id"), 10, 64)

	if err := h.userService.Delete(id, cu.ID); err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "删除成功",
	})
}

// CreateUser 管理员创建用户
func (h *Handler) CreateUser(c *gin.Context) {
	var req model.AdminCreateUserRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误: " + err.Error(),
		})
		return
	}

	user, err := h.userService.CreateUserByAdmin(req.Username, req.Password, req.IsAdmin)
	if err != nil {
		code := 500
		if err == service.ErrUserExists {
			code = 409
		}
		c.JSON(code, model.APIResponse{
			Code:    code,
			Message: err.Error(),
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "创建成功",
		Data:    user,
	})
}

// ToggleRegister 切换注册
func (h *Handler) ToggleRegister(c *gin.Context) {
	var req model.ToggleRegisterRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误",
		})
		return
	}

	h.userService.SetRegisterOpen(req.Enabled)

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "设置成功",
		Data: map[string]bool{
			"register_enabled": req.Enabled,
		},
	})
}

// GetSources 获取源列表
func (h *Handler) GetSources(c *gin.Context) {
	sources, err := h.sourceService.GetAll()
	if err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "获取源列表失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "success",
		Data:    sources,
	})
}

// AddSource 添加源
func (h *Handler) AddSource(c *gin.Context) {
	var req model.AddSourceRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误",
		})
		return
	}

	source := &model.VideoSource{
		Name:    req.Name,
		API:     req.API,
		Detail:  req.Detail,
		IsAdult: req.IsAdult,
		Enabled: true,
	}

	if err := h.sourceService.Create(source); err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "添加失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "添加成功",
		Data:    source,
	})
}

// UpdateSource 更新源
func (h *Handler) UpdateSource(c *gin.Context) {
	id, _ := strconv.ParseInt(c.Param("id"), 10, 64)

	source, err := h.sourceService.GetByID(id)
	if err != nil {
		c.JSON(http.StatusNotFound, model.APIResponse{
			Code:    404,
			Message: "源不存在",
		})
		return
	}

	var req model.UpdateSourceRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误",
		})
		return
	}

	if req.Name != "" {
		source.Name = req.Name
	}
	if req.API != "" {
		source.API = req.API
	}
	if req.Detail != "" {
		source.Detail = req.Detail
	}
	source.IsAdult = req.IsAdult
	if req.Enabled != nil {
		source.Enabled = *req.Enabled
	}

	if err := h.sourceService.Update(source); err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "更新失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "更新成功",
		Data:    source,
	})
}

// DeleteSource 删除源
func (h *Handler) DeleteSource(c *gin.Context) {
	id, _ := strconv.ParseInt(c.Param("id"), 10, 64)

	if err := h.sourceService.Delete(id); err != nil {
		c.JSON(http.StatusInternalServerError, model.APIResponse{
			Code:    500,
			Message: "删除失败",
		})
		return
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "删除成功",
	})
}

// GetConfig 获取配置
func (h *Handler) GetConfig(c *gin.Context) {
	config := h.settingService.GetConfig()

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "success",
		Data: map[string]interface{}{
			"config":           config,
			"register_enabled": h.userService.IsRegisterOpen(),
		},
	})
}

// UpdateConfig 更新配置
func (h *Handler) UpdateConfig(c *gin.Context) {
	var req struct {
		RegisterEnabled *bool `json:"register_enabled"`
	}

	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, model.APIResponse{
			Code:    400,
			Message: "参数错误",
		})
		return
	}

	if req.RegisterEnabled != nil {
		h.userService.SetRegisterOpen(*req.RegisterEnabled)
	}

	c.JSON(http.StatusOK, model.APIResponse{
		Code:    0,
		Message: "更新成功",
	})
}

// GetStats 获取统计
func (h *Handler) GetStats(c *gin.Context) {
	userCount, _ := h.userService.CountUsers()
	favCount, _ := h.userService.CountFavorites()
	historyCount, _ := h.userService.CountHistory()

	c.JSON(http.StatusOK, model.StatsResponse{
		Code:            0,
		TotalUsers:      userCount,
		TotalFavorites:  favCount,
		TotalHistory:    historyCount,
	})
}
