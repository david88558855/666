package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/katelyatv/katelyatv-go/internal/handler"
	"github.com/katelyatv/katelyatv-go/internal/repository"
	"github.com/katelyatv/katelyatv-go/internal/service"

	"github.com/gin-gonic/gin"
)

func main() {
	// 命令行参数
	port := flag.Int("port", 3000, "服务端口")
	dataDir := flag.String("data-dir", "./data", "数据存储目录")
	configPath := flag.String("config", "./config.json", "配置文件路径")
	flag.Parse()

	// 确保数据目录存在
	if err := os.MkdirAll(*dataDir, 0755); err != nil {
		log.Fatalf("创建数据目录失败: %v", err)
	}

	// 初始化数据库
	db, err := repository.NewDB(*dataDir)
	if err != nil {
		log.Fatalf("初始化数据库失败: %v", err)
	}
	defer db.Close()

	// 初始化仓库
	userRepo := repository.NewUserRepository(db)
	favRepo := repository.NewFavoriteRepository(db)
	historyRepo := repository.NewHistoryRepository(db)
	sourceRepo := repository.NewSourceRepository(db)
	settingRepo := repository.NewSettingRepository(db)
	cacheRepo := repository.NewCacheRepository(db)

	// 初始化服务
	userService := service.NewUserService(userRepo, favRepo, historyRepo)
	sourceService := service.NewSourceService(sourceRepo)
	settingService := service.NewSettingService(settingRepo, *configPath)
	cacheService := service.NewCacheService(cacheRepo)
	searchService := service.NewSearchService(sourceService, cacheService)

	// 初始化处理器
	h := handler.NewHandler(userService, sourceService, settingService, searchService)

	// 设置 Gin
	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()

	// 加载模板
	r.LoadHTMLGlob("web/templates/*.html")

	// 静态文件
	r.Static("/static", "./web/static")

	// 页面路由
	r.GET("/", h.Index)
	r.GET("/play", h.Play)
	r.GET("/search", h.SearchPage)
	r.GET("/admin", h.AdminPage)
	r.GET("/login", h.LoginPage)
	r.GET("/register", h.RegisterPage)
	r.GET("/settings", h.SettingsPage)

	// API 路由
	api := r.Group("/api")
	{
		// 认证
		auth := api.Group("/auth")
		{
			auth.POST("/register", h.Register)
			auth.POST("/login", h.Login)
			auth.POST("/logout", h.Logout)
		}

		// 公开 API
		api.GET("/categories", h.GetCategories)
		api.GET("/search", h.Search)
		api.GET("/detail/:site/:id", h.GetDetail)
		api.GET("/play/:site/:id", h.GetPlayUrl)
		api.GET("/home", h.GetHomeData)

		// TVBox
		api.GET("/tvbox", h.GetTVBoxConfig)

		// 需要认证的 API
		user := api.Group("/user")
		user.Use(h.AuthRequired())
		{
			user.GET("/info", h.GetUserInfo)
			user.GET("/favorites", h.GetFavorites)
			user.POST("/favorites", h.AddFavorite)
			user.DELETE("/favorites/:id", h.RemoveFavorite)
			user.GET("/history", h.GetHistory)
			user.POST("/history", h.AddHistory)
			user.PUT("/settings", h.UpdateUserSettings)
		}

		// 管理 API
		admin := api.Group("/admin")
		admin.Use(h.AdminRequired())
		{
			admin.GET("/users", h.GetUsers)
			admin.DELETE("/users/:id", h.DeleteUser)
			admin.PUT("/register", h.ToggleRegister)
			admin.GET("/sources", h.GetSources)
			admin.POST("/sources", h.AddSource)
			admin.PUT("/sources/:id", h.UpdateSource)
			admin.DELETE("/sources/:id", h.DeleteSource)
			admin.GET("/config", h.GetConfig)
			admin.PUT("/config", h.UpdateConfig)
			admin.GET("/stats", h.GetStats)
		}
	}

	// 启动服务
	addr := fmt.Sprintf(":%d", *port)
	log.Printf("KatelyaTV-Go 启动中，端口: %d", *port)
	log.Printf("数据目录: %s", *dataDir)
	log.Printf("配置文件: %s", *configPath)

	// 优雅关闭
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		if err := r.Run(addr); err != nil {
			log.Fatalf("服务启动失败: %v", err)
		}
	}()

	<-quit
	log.Println("正在关闭服务...")
}
