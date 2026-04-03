package web

import (
	"embed"
	"html/template"
	"io/fs"
	"log"
	"net/http"
)

// Templates 嵌入 HTML 模板
//
//go:embed templates/*.html
var Templates embed.FS

// Static 嵌入静态资源
//
//go:embed static/css/* static/js/*
var Static embed.FS

// BuildHTMLTemplate 从嵌入文件构建模板集
func BuildHTMLTemplate() *template.Template {
	templFS, err := fs.Sub(Templates, "templates")
	if err != nil {
		log.Fatalf("读取模板失败: %v", err)
	}
	t := template.New("")
	err = fs.WalkDir(templFS, ".", func(path string, d fs.DirEntry, err error) error {
		if err != nil || d.IsDir() {
			return err
		}
		data, err := fs.ReadFile(templFS, path)
		if err != nil {
			return err
		}
		t, err = t.New(path).Parse(string(data))
		return err
	})
	if err != nil {
		log.Fatalf("解析模板失败: %v", err)
	}
	return t
}

// StaticFS 返回静态文件 http.FileSystem
func StaticFS() http.FileSystem {
	sub, _ := fs.Sub(Static, "static")
	return http.FS(sub)
}
