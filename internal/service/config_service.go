package service

import (
	"encoding/json"
	"os"

	"github.com/katelyatv/katelyatv-go/internal/model"
)

// LoadAppConfig 加载应用配置
func LoadAppConfig(path string) *model.AppConfig {
	config := &model.AppConfig{
		CacheTime: 7200,
		APISite:  make(map[string]model.APISite),
	}

	data, err := os.ReadFile(path)
	if err != nil {
		// 如果文件不存在，返回默认配置
		return config
	}

	if err := json.Unmarshal(data, config); err != nil {
		// 解析失败，返回默认配置
		return config
	}

	return config
}

// SaveAppConfig 保存应用配置
func SaveAppConfig(path string, config *model.AppConfig) error {
	data, err := json.MarshalIndent(config, "", "  ")
	if err != nil {
		return err
	}

	// 确保目录存在
	dir := path[:len(path)-len("/config.json")]
	if dir == path {
		dir = "./"
	}
	os.MkdirAll(dir, 0755)

	return os.WriteFile(path, data, 0644)
}

// DefaultConfig 默认配置
var DefaultConfig = &model.AppConfig{
	CacheTime: 7200,
	APISite: map[string]model.APISite{
		"example": {
			API:     "https://api.example.com/provide/vod",
			Name:    "示例资源站",
			Detail:  "https://example.com",
			IsAdult: false,
		},
	},
}
