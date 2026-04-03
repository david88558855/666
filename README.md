# KatelyaTV-Go

基于 Go 开发的影视聚合播放器，重构自 [KatelyaTV](https://github.com/haogege8888/KatelyaTV)。

## 功能特性

- 🔍 **聚合搜索**：整合多个影视资源站，一键搜索
- 📺 **高清播放**：支持多种视频格式播放
- ⏭️ **智能跳过**：自动跳过片头片尾
- 🎯 **断点续播**：记录播放进度
- ⭐ **收藏功能**：收藏喜欢的影视
- 👥 **多用户支持**：独立用户系统
- 🔒 **内容过滤**：成人内容过滤
- 📱 **TVBox 兼容**：支持 TVBox 配置接口

## 快速开始

### 下载二进制

从 GitHub Releases 下载编译好的 Linux amd64 musl 二进制文件：

```bash
# 下载最新版本
wget https://github.com/你的用户名/katelyatv-go/releases/latest/download/katelyatv-go_linux_amd64_musl.tar.gz
tar -xzf katelyatv-go_linux_amd64_musl.tar.gz
./katelyatv-go --port 3000
```

### 从源码编译

```bash
git clone https://github.com/你的用户名/katelyatv-go.git
cd katelyatv-go
go build -tags "sqlite_fts5" -ldflags="-w -s" -o katelyatv-go ./cmd/server
```

### Docker 运行

```bash
docker run -d \
  --name katelyatv \
  -p 3000:3000 \
  -v $(pwd)/data:/app/data \
  katelyatv-go:latest
```

## 配置说明

### 命令行参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--port` | 3000 | 服务端口 |
| `--data-dir` | ./data | 数据存储目录 |
| `--config` | ./config.json | 配置文件路径 |

### 配置文件 (config.json)

```json
{
  "cache_time": 7200,
  "api_site": {
    "site1": {
      "api": "https://api.example.com/provide/vod",
      "name": "示例资源站",
      "is_adult": false
    }
  }
}
```

## 用户系统

- **第一个注册用户**：自动成为管理员
- **注册后**：默认关闭注册（管理员可手动开启）
- **管理员权限**：添加/删除/编辑视频源，管理用户

## API 接口

### 认证

- `POST /api/auth/register` - 用户注册
- `POST /api/auth/login` - 用户登录
- `POST /api/auth/logout` - 用户登出

### 视频

- `GET /api/search?q=关键词` - 搜索视频
- `GET /api/categories` - 获取分类
- `GET /api/source/:id` - 获取视频源详情
- `GET /api/detail/:id` - 获取视频详情

### 用户

- `GET /api/user/favorites` - 获取收藏
- `POST /api/user/favorites` - 添加收藏
- `DELETE /api/user/favorites/:id` - 删除收藏
- `GET /api/user/history` - 获取历史记录
- `POST /api/user/history` - 添加历史记录

### 管理 (需管理员权限)

- `GET /api/admin/users` - 获取用户列表
- `DELETE /api/admin/users/:id` - 删除用户
- `PUT /api/admin/register` - 开启/关闭注册
- `GET /api/admin/sources` - 获取视频源列表
- `POST /api/admin/sources` - 添加视频源
- `PUT /api/admin/sources/:id` - 更新视频源
- `DELETE /api/admin/sources/:id` - 删除视频源
- `GET /api/admin/config` - 获取配置
- `PUT /api/admin/config` - 更新配置

### TVBox

- `GET /api/tvbox?format=json` - JSON 格式配置
- `GET /api/tvbox?format=txt` - TXT 格式配置

## 数据存储

- **SQLite**：用户数据、收藏、历史记录、配置
- **文件系统**：缓存数据

## License

MIT
