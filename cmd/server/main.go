package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/gin-gonic/gin"
	"github.com/katelyatv/katelyatv-go/internal/handler"
	"github.com/katelyatv/katelyatv-go/internal/repository"
	"github.com/katelyatv/katelyatv-go/internal/service"
	"github.com/katelyatv/katelyatv-go/web"
)

func main() {
	port := flag.Int("port", 3000, "服务端口")
	dataDir := flag.String("data-dir", "./data", "数据存储目录")
	configPath := flag.String("config", "./config.json", "配置文件路径")
	flag.Parse()

	if err := os.MkdirAll(*dataDir, 0755); err != nil {
		log.Fatalf("创建数据目录失败: %v", err)
	}

	db, err := repository.NewDB(*dataDir)
	if err != nil {
		log.Fatalf("初始化数据库失败: %v", err)
	}
	defer db.Close()

	userRepo := repository.NewUserRepository(db)
	favRepo := repository.NewFavoriteRepository(db)
	historyRepo := repository.NewHistoryRepository(db)
	sourceRepo := repository.NewSourceRepository(db)
	settingRepo := repository.NewSettingRepository(db)
	cacheRepo := repository.NewCacheRepository(db)

	userService := service.NewUserService(userRepo, favRepo, historyRepo)
	sourceService := service.NewSourceService(sourceRepo)
	settingService := service.NewSettingService(settingRepo, *configPath)
	cacheService := service.NewCacheService(cacheRepo)
	searchService := service.NewSearchService(sourceService, cacheService)

	h := handler.NewHandler(userService, sourceService, settingService, searchService)

	gin.SetMode(gin.ReleaseMode)
	r := gin.Default()

	// 使用嵌入的模板和静态文件
	r.SetHTMLTemplate(web.BuildHTMLTemplate())
	r.StaticFS("/static", web.StaticFS())

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
		auth := api.Group("/auth")
		{
			auth.POST("/register", h.Register)
			auth.POST("/login", h.Login)
			auth.POST("/logout", h.Logout)
		}

		api.GET("/categories", h.GetCategories)
		api.GET("/search", h.Search)
		api.GET("/detail", h.GetDetail)       // 使用查询参数 ?site=&id=
		api.GET("/play", h.GetPlayUrl)         // 使用查询参数 ?site=&id=&episode=
		api.GET("/home", h.GetHomeData)
		api.GET("/tvbox", h.GetTVBoxConfig)

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

		admin := api.Group("/admin")
		admin.Use(h.AuthRequired(), h.AdminRequired())
		{
			admin.GET("/users", h.GetUsers)
			admin.POST("/users", h.CreateUser)
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

	addr := fmt.Sprintf(":%d", *port)
	log.Printf("KatelyaTV-Go 启动中，端口: %d", *port)
	log.Printf("数据目录: %s", *dataDir)
	log.Printf("配置文件: %s", *configPath)

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
