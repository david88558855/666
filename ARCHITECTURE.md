# gostc-rs 架构方案

> 基于 [SianHH/gostc-open](https://github.com/SianHH/gostc-open)（Apache-2.0）的 Rust 重写项目架构方案。
> 本文档是 **设计提案**，不包含最终代码实现。代码实施需经评审通过后再开始。

---

## 1. 背景与目标

### 1.1 背景

`gostc-open` 是基于 FRP 衍生版（`SianHH/frp-package`）的 Go 内网穿透管理平台，包含 4 个组件：管理服务端、节点/客户端、网关、前端控制台。代码规模：

| 组件 | 语言 | 文件数 | 行数（粗略） |
|---|---|---|---|
| `server`（管理后端） | Go | 386 | 4 829 |
| `client`（节点/客户端） | Go | 98 | 5 351 |
| `proxy`（网关） | Go | 32 | 1 732 |
| `web`（前端） | Vue | 133 | — |

依赖：Gin（HTTP 框架）、arpx（自定义 RPC）、JWT、Cobra、SQLite/MySQL、Go-Gost x 等。底层 FRP 隧道通过 `SianHH/frp-package` 引入（FRP 本身的 50k+ 行 Go 不直接出现在仓库）。

### 1.2 目标

用 **Rust** 重写一套等价的内网穿透管理平台，代号 `gostc-rs`，提供：

1. 与 gostc-open **功能等价的核心能力**：多用户、多节点、隧道注册、运行时配置、节点认证
2. **更高的运行时效率**（Rust 异步生态 + 单二进制部署）
3. **更小的体积**（单 binary ≤ 10 MB，参考 rathole ~500KB 的水平做减法）
4. **协议可选**：原生 Rust 协议（基于 rathole 兼容），不强制兼容 FRP 协议以避免引入 FRP 整套 Go 实现

### 1.3 非目标

- 不复刻 gostc-open 的全部商业版特性（CDK、易支付、用户组套餐）。这些是商业运营特性，不属于开源版应有范围
- 不实现 Windows GUI 客户端（gostc-open 的 `client/gui` 是 Gio 实现的桌面壳，第一版不需要）
- 不替代 FRP 官方客户端。要兼容老 FRP 客户端，需另起项目维护协议层

---

## 2. 总体架构

```
                  ┌──────────────────────────────────────────────┐
                  │  管理后端 gostc-rs-admin (Rust + axum)         │
                  │  - REST API + WebSocket 控制面                  │
                  │  - 用户/节点/隧道/限速 CRUD                       │
                  │  - 持久化：SQLite（默认）/ PostgreSQL（可选）     │
                  │  - 鉴权：JWT（短期）+ API Key                    │
                  │  - 静态托管 web/（管理后台）                       │
                  └──────┬──────────────────────┬──────────────────┘
                         │ 控制面 (HTTP/WS, JWT)  │ 配置下发 (HTTP/WS)
                         ▼                      ▼
        ┌─────────────────────────┐  ┌─────────────────────────────┐
        │ 节点 gostc-rs-tunnel-   │  │ 网关 gostc-rs-gateway (Rust) │
        │ server (Rust, rathole)  │  │  - 自定义域名路由             │
        │  - 监听控制面连接        │  │  - TLS 终止                  │
        │  - 拉取隧道配置          │  │  - 转发到 tunnel-server      │
        │  - 对外暴露隧道端口       │  └──────────────┬──────────────┘
        └──────┬──────────────────┘                 │
               │ 数据面 (TCP/UDP, token auth)        │
               ▼                                    │
        ┌─────────────────────────┐                 │
        │ 客户端 gostc-rs-tunnel- │ ────────────────┘
        │ client (Rust, rathole)  │   反向连接到最近的节点
        │  - 长连接到节点          │
        │  - 提供 local_addr 服务  │
        └─────────────────────────┘
```

**两平面分离**：
- **控制面**：节点/客户端通过 HTTPS + JWT 与 `admin` 通信（配置、心跳、状态上报）
- **数据面**：终端用户访问 `tunnel-server` 暴露的端口，数据通过 `tunnel-client` 反向通道回到内网服务

---

## 3. 技术选型

### 3.1 传输层（隧道）

> 命名澄清：本节提到的 **orbien** 指 [`orbien-org/orbien`](https://github.com/orbien-org/orbien)（纯 Rust，Apache-2.0，v3.0.0，5MB binary）。还有一个同名仓库 [`lxien/orbien`](https://github.com/lxien/orbien) 是 Java Netty 服务端 + 小 Rust 客户端的混合栈，不适合本项目"全 Rust"的目标，**不在本方案参照中**。

| 选项 | 评价 |
|---|---|
| **orbien**（推荐） | 全 Rust，Tokio + yamux + quinn + rustls + kcp-tokio + axum；传输支持 TCP / WebSocket / QUIC / KCP；应用层支持 TCP / UDP / HTTP / HTTPS / SOCKS5 / 文件共享；自带 Web 管理 dashboard；token + mTLS；与 gostc-open 的特性重合度最高 |
| rathole | 14.1k stars，体积 ~500KB 更小；但传输只支持 TCP/TLS/Noise/WS，不支持 QUIC/KCP，对 gostc-open 特性覆盖度低；作为更轻量的备选保留 |
| 自研 tokio + yamux + rustls | 完全可控但工作量量大；MVP 不必要 |

**结论**：MVP 阶段 fork `orbien-org/orbien` 为内部 crate，**先以子进程方式**调用 orbien 的 `orbien-server` / `orbien` 二进制（写 TOML + SIGHUP reload），不深度改 orbien 源码。中后期视需求，将 orbien 的核心库化为内部 crate，让 admin 进程直接控制其生命周期（减少一个进程）。

### 3.2 管理后端

| 依赖 | 用途 |
|---|---|
| `axum` 0.7+ | HTTP 框架（相比 actix-web，生态更易与 tower 组合） |
| `tokio` 1.x | 异步运行时 |
| `sqlx` | 编译期 SQL 检查，支持迁移 |
| `sea-orm` 或 `diesel` | ORM（待 Phase 2 选型） |
| `jsonwebtoken` | JWT 签发/校验 |
| `argon2` | 密码哈希 |
| `serde` + `serde_json` | 序列化 |
| `tracing` + `tracing-subscriber` | 结构化日志 |
| `rustls` | TLS |
| `utoipa` | OpenAPI 文档自动生成 |

### 3.3 网关

`hyper` 或 `pingora`（Cloudflare 开源的反向代理框架，基于 tokio）。Phase 1 用 `hyper` 简化实现，Phase 3 视性能需求切换 pingora。

### 3.4 前端

参考 gostc-open 的 Vue 实现，新项目**第一版不强制要求 web 后台**——可使用 `cargo run -- admin` 自带的 Swagger UI（utoipa + utoipa-swagger-ui）作为控制面入口，web 后台作为 Phase 4 任务。

如果实现 web 后台，建议 **Vue 3 + Vite + Pinia + Element Plus**（沿用 gostc-open 栈减少迁移成本），或换 React + shadcn/ui（更现代）。

### 3.5 持久化

- 默认 SQLite（`sqlx` + `sqlite` feature），单文件部署
- 可选 PostgreSQL（`sqlx` + `postgres` feature），适合多节点部署
- 数据库迁移：`sqlx migrate`

### 3.6 配置

- 管理后端：`config.toml`（类似 rathole 风格），支持通过控制面热修改运行时配置
- 节点/客户端：`config.toml` 从管理后端拉取，本地缓存

---

## 4. 仓库结构

```
gostc-rs/                                  # GitHub: david88558855/666
├── Cargo.toml                             # workspace 根
├── rust-toolchain.toml                    # 固定 stable 工具链版本（CI 一致性）
├── README.md                              # 项目入口
├── ARCHITECTURE.md                        # 本文档
├── LICENSE                                # Apache-2.0
├── NOTICE                                 # 归属声明
├── .github/
│   └── workflows/
│       ├── rust-build.yml                 # 编译 + clippy + fmt
│       └── release.yml                    # （Phase 5） tag 触发多平台 release
├── crates/
│   ├── common/                            # 共享类型：协议消息、错误、配置
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── admin/                             # 管理后端二进制
│   │   ├── Cargo.toml
│   │   ├── migrations/                    # sqlx 迁移
│   │   └── src/
│   │       ├── main.rs
│   │       ├── api/                       # REST 路由
│   │       ├── auth/                      # JWT + 权限
│   │       ├── db/                        # sqlx models
│   │       ├── control/                   # 控制面 WS（节点/客户端上报）
│   │       └── web/                       # 静态文件托管
│   ├── tunnel-server/                     # 隧道服务端二进制
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── tunnel-client/                     # 隧道客户端二进制
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── gateway/                           # 自定义域名网关
│       ├── Cargo.toml
│       └── src/main.rs
├── web/                                  # （Phase 4）前端源码
└── docs/                                 # 文档站点
```

四个二进制产物：`gostc-rs-admin`、`gostc-rs-tunnel-server`、`gostc-rs-tunnel-client`、`gostc-rs-gateway`。

---

## 5. 数据模型（核心表）

```sql
-- 用户
CREATE TABLE users (
    id          INTEGER PRIMARY KEY,
    username    TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,        -- argon2
    role        TEXT NOT NULL,           -- 'admin' | 'user'
    traffic_quota_bytes INTEGER,        -- 流量配额（NULL = 无限）
    bandwidth_limit_bps INTEGER,         -- 带宽限制
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 节点（tunnel-server 实例）
CREATE TABLE nodes (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL,
    secret          TEXT UNIQUE NOT NULL,  -- 节点密钥
    api_endpoint    TEXT NOT NULL,         -- 控制面地址（admin <-> node）
    tunnel_endpoint TEXT NOT NULL,         -- 数据面地址（client -> node）
    status          TEXT NOT NULL,         -- online | offline | disabled
    last_heartbeat  TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 隧道（client 通过 node 暴露的内服）
CREATE TABLE tunnels (
    id            INTEGER PRIMARY KEY,
    user_id       INTEGER NOT NULL REFERENCES users(id),
    node_id       INTEGER NOT NULL REFERENCES nodes(id),
    name          TEXT NOT NULL,
    type          TEXT NOT NULL,           -- tcp | udp | http | https
    local_addr    TEXT NOT NULL,           -- 127.0.0.1:22
    remote_port   INTEGER,                 -- 节点暴露端口（type=tcp/udp）
    domain        TEXT,                    -- type=https/http 时的域名
    token         TEXT NOT NULL,             -- 隧道级 token
    status        TEXT NOT NULL,           -- active | paused
    created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 流量统计
CREATE TABLE traffic_logs (
    id          INTEGER PRIMARY KEY,
    tunnel_id   INTEGER NOT NULL REFERENCES tunnels(id),
    bytes_in    INTEGER NOT NULL,
    bytes_out   INTEGER NOT NULL,
    recorded_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 审计日志
CREATE TABLE audit_logs (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER REFERENCES users(id),
    action      TEXT NOT NULL,
    target      TEXT,
    details     TEXT,                       -- JSON
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

索引：`traffic_logs(tunnel_id, recorded_at)`、`tunnels(user_id)`、`nodes(secret)`。

---

## 6. API 设计（管理面，REST）

所有响应统一 JSON。鉴权：HTTP Bearer JWT（`Authorization: Bearer <token>`），从 `/api/v1/auth/login` 获取。

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api/v1/auth/login` | 用户登录，返回 access + refresh token |
| POST | `/api/v1/auth/refresh` | 刷新 access token |
| GET/POST/PUT/DELETE | `/api/v1/users[/:id]` | 用户 CRUD（仅 admin） |
| GET/POST/PUT/DELETE | `/api/v1/nodes[/:id]` | 节点 CRUD |
| GET/POST/PUT/DELETE | `/api/v1/tunnels[/:id]` | 隧道 CRUD |
| GET | `/api/v1/tunnels/:id/traffic` | 流量历史 |
| GET | `/api/v1/audit-logs` | 审计日志（仅 admin） |

控制面 WS（节点 ↔ admin）：`/ws/control`，握手时通过节点 secret 鉴权。

完整 OpenAPI 文档由 `utoipa` 自动生成在 `/swagger-ui/`。

---

## 7. 协议层（控制面 / 数据面）

### 7.1 控制面

- 节点和客户端通过 HTTPS 长连接到 admin 的 `/ws/control`
- 协议：WebSocket + JSON 消息
- 消息类型：`Hello`（带 secret）、`ConfigPull`（拉配置）、`ConfigUpdate`（配置变更推送）、`Heartbeat`、`TunnelStatus`

### 7.2 数据面（隧道）

- 直接复用 **orbien** 协议：orbien 的 wire protocol 在 `orbien-org/orbien` 源码 `client-rs/src/` 和 `server/src/` 下，基于 yamux 多路复用 + 自定义消息帧
- 第一次集成时，把 orbien 作为黑盒 binary 调用，通过 SIGHUP 重载配置
- 中后期目标：把 orbien 的核心库化为内部 crate（特别是 `orbien-core` crate 中已抽出的协议层），让 admin 进程直接控制其生命周期（减少一个进程、避免 TOML round-trip）

---

## 8. 部署形态

### 8.1 单机部署（个人 / 小团队）

所有 4 个二进制部署在同一台公网机器：

```
[公网 IP:8080]  gostc-rs-admin            # 管理后台
[公网 IP:2333]  gostc-rs-tunnel-server    # 节点监听 client
[公网 IP:443/4443] gostc-rs-gateway       # 自定义域名
```

`gostc-rs-tunnel-client` 部署在内网机器，连接节点。

### 8.2 Docker Compose（推荐）

```yaml
version: "3"
services:
  admin:
    image: david88558855/gostc-rs-admin:latest
    network_mode: host
    volumes: [./data/admin:/data]
  node:
    image: david88558855/gostc-rs-tunnel-server:latest
    network_mode: host
    depends_on: [admin]
  client:                                # 部署在内网机器
    image: david88558855/gostc-rs-tunnel-client:latest
    network_mode: host
    environment:
      ADMIN_ADDR: admin.example.com:8080
      NODE_SECRET: ...
```

### 8.3 二进制分发

CI 阶段（`release.yml`，Phase 5 启用）打 4 个平台的二进制包：
- `x86_64-unknown-linux-gnu`（musl）
- `aarch64-unknown-linux-musl`
- `x86_64-pc-windows-msvc`
- `x86_64-apple-darwin`

---

## 9. 实施阶段

### Phase 1：基础设施（本仓库已部分完成）

- [x] Cargo workspace 骨架（4 个 crate + root `Cargo.toml`）
- [x] GitHub Actions `rust-build.yml`：build / clippy / fmt
- [ ] 共享 crate `common`：协议消息类型、错误定义
- [ ] `admin` 启动框架：axum + 配置加载 + 健康检查 `/healthz`

### Phase 2：管理后端 MVP

- [ ] 用户表 CRUD + JWT 鉴权 + 登录/刷新
- [ ] 节点表 CRUD
- [ ] 隧道表 CRUD
- [ ] 控制面 WS（节点 hello + heartbeat）
- [ ] 流量上报 API + 简单统计

### Phase 3：隧道打通

- [ ] 集成 rathole（fork 子进程方案）
- [ ] 节点端：拉取本节点隧道列表 → 写 rathole server.toml → SIGHUP
- [ ] 客户端：拉取本客户端隧道列表 → 写 rathole client.toml → 启动 rathole
- [ ] 速率限制：在控制面按 tunnel.token 限速，rathole 端做 token 校验

### Phase 4：网关与前端

- [ ] `gateway` 实现 SNI + Host 路由到节点
- [ ] 自动 HTTPS（ACME）
- [ ] Web 后台（Vue 3 + Vite）
- [ ] 集成 Swagger UI

### Phase 5：发布与运营

- [ ] `release.yml`：tag 触发多平台构建 + GitHub Release
- [ ] Docker 镜像发布到 ghcr.io / Docker Hub
- [ ] 文档站（docs.rs / 自建 mkdocs）

---

## 10. 许可证与归属

本项目采用 **Apache-2.0** 许可证，与上游 `gostc-open` 一致。`NOTICE` 文件需包含：

- 上游项目 `SianHH/gostc-open`（Apache-2.0）— 设计与功能参照来源
- 上游项目 `rathole-org/rathole`（MIT 或 Apache-2.0）— 传输层基线
- 上游项目 `fatedier/frp`（Apache-2.0）— 协议设计参考

**合规要求**（来自 Apache-2.0 第 4 条）：

1. 保留上游版权声明
2. 显著标注所有修改
3. 不得使用上游贡献者姓名为衍生作品背书
4. 若修改 NOTICE 文件，须随附修改说明

`LICENSE` 与 `NOTICE` 文件在仓库根目录。

---

## 11. 风险与开放问题

| 风险 / 问题 | 影响 | 缓解 |
|---|---|---|
| orbien 目前没有运行时 HTTP API，无法动态增删隧道 | Phase 3 必须 fork orbien 或子进程方案 | 先子进程，Phase 3 中期 fork 化；优先 fork `orbien-core` crate（已是独立协议层） |
| gostc-open 的 frp-package 是 SianHH 私有 fork | 无法直接参考其定制代码 | 走 orbien 协议，避免依赖 frp |
| `arpx`（gostc-open 的控制面 RPC）是定制协议 | 我们用 WebSocket + JSON 简化 | 协议是内部接口，重新设计即可 |
| Rust 跨平台编译（musl + Windows）依赖工具链 | CI 已配置，开发者本机需 rustup | 文档说明 |
| 多用户流量配额与统计性能 | 高 QPS 下 `traffic_logs` 写入压力大 | 周期性聚合 + 时序采样（Phase 4+） |
| gostc-open 的 web 资源以 zip 嵌入 Go binary | 我们改为 axum 静态托管 | Phase 4 完成 |
| orbien 仓库命名歧义（lxien/orbien vs orbien-org/orbien） | 误导后续维护者 | 本节已明确锁定 `orbien-org/orbien`；NOTICE 与 README 同此 |

---

## 12. 评审清单

进入代码实施前需要确认：

- [ ] 总体架构图是否符合预期
- [ ] 传输层基线选择 orbien（vs rathole / 自研）是否同意
- [ ] 数据模型是否够用
- [ ] 是否需要兼容官方 FRP 客户端（决定协议路线）
- [ ] Phase 1 之后的优先级：管理后端 / 隧道打通 / 网关
- [ ] 许可证与归属声明文本

---

_文档版本：v0.2 — 2026-09-29_
_变更：传输层基线由 rathole 调整为 orbien（orbien-org/orbien），同步更新协议章节、依赖章节、风险章节、NOTICE 归属。_