# gostc-rs 架构方案

> 基于 [SianHH/gostc-open](https://github.com/SianHH/gostc-open)（Apache-2.0）的 Rust 重写项目。
> 对齐策略：**管理面体验对齐 gostc-open，数据面协议对齐 [orbien-org/orbien](https://github.com/orbien-org/orbien)**。

---

## 1. 背景与目标

### 1.1 背景

`gostc-open` 是基于 FRP 衍生版的 Go 内网穿透管理平台：Web 控制台统一管理用户、节点、客户端与隧道，节点与客户端都只连中央控制台，启动没有先后顺序，配置在面板修改后自动下发。

### 1.2 目标与对齐基线

用 **Rust** 重写 gostc（避免重写 frp 的时间成本），代号 `gostc-rs`。对齐策略：

- **gostc-open 是产品蓝本**：面板体验、多用户/多节点/客户端/隧道模型、配置下发 —— 全部按 gostc 复刻
- **orbien 是数据面底层**：`orbien-org/orbien` 本身就是 Rust 实现的 frp 等价物（TCP/UDP/HTTP/HTTPS/SOCKS5 隧道，TCP+yamux/QUIC/WebSocket/KCP 传输，token/mTLS），**直接复用其底层**，不重写隧道协议栈

| 层 | 对齐对象 | 对齐方式 |
|---|---|---|
| **管理面（控制台）** | gostc-open | 面板版块/交互/术语：登录、全站统计、节点管理、客户端、私有隧道、用户管理；节点/客户端创建即给出运行命令；隧道改动自动下发 |
| **数据面（隧道协议）** | orbien-org/orbien | **直接复用**：core 消息协议 + 传输栈以库/子进程形式嵌入 tunnel-server 与 tunnel-client（见 §7.6 集成路线） |

非目标：

- 不兼容 FRP 官方客户端协议
- 不复刻 gostc-open 商业版特性（CDK、易支付、用户组套餐）
- ~~自定义域名网关 `gostc-rs-gateway`~~（**已从范围移除**；HTTP/HTTPS 域名路由由节点内置实现，对齐 orbien 的 `http.rs`/`https.rs` 隧道）

---

## 2. 总体架构（当前已实现形态）

```
                  ┌────────────────────────────────────────────────┐
                  │   中央控制台 gostc-rs-admin (Rust + axum)        │
                  │   - REST API + 内嵌单文件中文 Web 面板            │
                  │   - 用户 / 节点 / 客户端 / 隧道 CRUD              │
                  │   - SQLite（sqlx 迁移）+ JWT + argon2            │
                  └──────┬─────────────────────────┬────────────────┘
             注册/心跳/拉配置 │ HTTP（30s 轮询）         │ HTTP（30s 轮询）
                          ▼                        ▼
        ┌──────────────────────────┐   ┌──────────────────────────┐
        │ 节点 gostc-rs-tunnel-    │   │ 客户端 gostc-rs-tunnel-   │
        │ server（公网服务器）        │   │ client（内网机器）          │
        │ - 按隧道配置开放公网端口    │◄──┼─ 反向数据连接（连接池）      │
        │ - ingress 分发            │   │ - 连接 local_addr 服务     │
        └──────────────────────────┘   └──────────────────────────┘
                  ▲  数据面（orbien 对齐协议）
                  └── 终端用户访问 节点公网IP:remote_port
```

**连接模型（与 gostc 一致）**：

- 节点与客户端**都只连中央控制台**，启动无先后顺序
- 客户端只需 `--api <面板地址> --token <客户端Token>`，无需知道节点地址
- **绑定关系在隧道上**：创建隧道时选「客户端 + 节点」
- 心跳即配置下发：节点 30s 重注册、客户端 30s 轮询，面板改动自动生效无需重启

---

## 3. 技术选型

| 领域 | 选型 | 说明 |
|---|---|---|
| 管理后端 | axum 0.7 + tokio | REST + 静态托管 |
| 持久化 | sqlx 0.8 + SQLite | 单文件部署，编译期迁移 |
| 鉴权 | jsonwebtoken(HS256) + argon2 | 管理面 JWT；节点/客户端用 secret/token |
| 数据面协议 | **直接复用 orbien 底层** | core 消息协议与传输栈嵌入 tunnel-server / tunnel-client；集成路线见 §7.6 |
| 传输栈（规划） | tokio + yamux + rustls + quinn + kcp-tokio | 与 orbien 同栈，见 §7.4 支持矩阵 |
| 前端 | 内嵌单文件 HTML（include_str!） | 无外部 CDN，air-gapped 可用；版块对齐 gostc |

> 命名澄清：**orbien** 指 [`orbien-org/orbien`](https://github.com/orbien-org/orbien)（纯 Rust，Apache-2.0）。同名仓库 [`lxien/orbien`](https://github.com/lxien/orbien)（Java + Rust 混合栈）不在参照中。

---

## 4. 仓库结构

```
gostc-rs/                                  # GitHub: david88558855/666
├── Cargo.toml                             # workspace 根
├── README.md / ARCHITECTURE.md / LICENSE / NOTICE
├── .github/workflows/rust-build.yml       # 三平台编译 + clippy
├── crates/
│   ├── common/                            # 共享类型（协议消息、错误、配置）
│   ├── admin/                             # 中央控制台
│   │   ├── migrations/                    # sqlx 迁移（001~003）
│   │   └── src/
│   │       ├── api/                       # auth/clients/nodes/tunnels/users
│   │       ├── extractors.rs              # AuthUser / AdminUser
│   │       ├── config.rs                  # load_or_create（首启自动生成）
│   │       ├── db.rs                      # 池 + 行模型
│   │       └── web/                       # index.html（内嵌面板）
│   ├── tunnel-server/                     # 节点二进制
│   │   └── src/{main.rs, api_client.rs}
│   └── tunnel-client/                     # 客户端二进制
│       └── src/{main.rs, api_client.rs}
```

三个二进制产物：`gostc-rs-admin`、`gostc-rs-tunnel-server`、`gostc-rs-tunnel-client`。

---

## 5. 数据模型（已实现）

核心表：`users`、`nodes`、`clients`、`tunnels`（+ 规划中 `traffic_logs`、`audit_logs`）。

- `clients`：客户端凭据实体（`token` 48 hex），**不绑定节点**（迁移 003 起 `node_id` 可空，仅为兼容保留）
- `tunnels`：`client_id` + `node_id` 双外键 —— 隧道是绑定关系的载体；`type`（tcp/udp/http/https）、`local_addr`、`remote_port`、`domain`、`token`、`status`

---

## 6. API（管理面，已实现）

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/auth/login` | 登录（admin/admin 首启默认，返回 JWT + must_change_password） |
| POST | `/auth/password` | 修改密码 |
| GET/POST/DELETE | `/users[/:id]` | 用户管理（admin） |
| GET/POST/DELETE | `/nodes[/:id]` | 节点管理；GET `:id/secret` 重取节点命令 |
| POST | `/nodes/register` | 节点注册+心跳（secret 认证），返回该节点全部隧道+所属客户端 |
| GET/POST/DELETE | `/clients[/:id]` | 客户端管理（创建只需 name） |
| POST | `/clients/connect` | 客户端连接（token 认证），返回其 active 隧道涉及的节点端点列表 |
| GET/POST/PATCH/DELETE | `/tunnels[/:id]` | 隧道 CRUD（client_id + node_id） |

---

## 7. 数据面（orbien 底层）

> 定位：orbien（Rust 实现的 frp 等价物）是本项目**直接复用的底层**，不是重写对象。
> 以下 §7.1–7.5 是其协议快照（理解与排障用），§7.6 是嵌入 gostc-rs 的集成路线，§7.7 是与 gostc-open 的功能差距清单。

### 7.0 传输协议选型（节点级配置）

与 orbien 的 `[transport] protocol` 对应：**每个节点在面板创建/编辑时固定一种传输协议**，客户端连接该节点时自动使用同一协议：

| 协议 | 说明 | 节点侧端口 | 现状 |
|---|---|---|---|
| `tcp`（默认） | TCP，可叠 yamux 复用 | `listen` 单端口 | ✅ 字段/面板/API 已通；转发为 MVP 简化协议 |
| `quic` | quinn，抗丢包、原生多流 | `quicPort` | ✅ 配置链路已通；数据面待 orbien 接入 |
| `websocket` | HTTP upgrade，可过 CDN | `listen` + `wsPath` | ✅ 配置链路已通；数据面待 orbien 接入 |
| `kcp` | kcp-tokio，弱网加速 | `kcpPort` | ✅ 配置链路已通；数据面待 orbien 接入 |

### 7.1 协议总览

orbien 的隧道协议是「**控制连接 + 独立数据连接池**」模型（与 frp 同源）：

- **控制连接**：长连接，只跑消息帧（登录、隧道注册、数据连接请求、心跳）
- **数据连接**：独立建立的裸流（或复用流），池化预建；每个公网连接从池中取出一条，先写路由头再转发
- 好处：控制面阻塞不影响数据转发；数据流是纯透传，无帧开销

### 7.2 消息帧（wire format）

```
+------------+-----------------+----------------+
| type (1B)  | length (u32 LE) | JSON body      |
+------------+-----------------+----------------+
```

- length 上限 4 MiB（防恶意超长帧）
- 消息类型字节（对齐 orbien `core/src/msg/types.rs`）：

| 类型 | 字节 | 方向 | 载荷要点 |
|---|---|---|---|
| Login | `A` | C→S | version, hostname, os, arch, user, agent_id, auth_digest, timestamp, session_id, pool_count |
| LoginResp | `a` | S→C | version, session_id, error |
| NewTunnel | `T` | C→S | tunnel_name, protocol, remote_port, local_ip, local_port, domains, locations, basic_auth_*, headers, bandwidth, bandwidth_limit_side |
| NewTunnelResp | `t` | S→C | tunnel_name, remote_addr, error |
| CloseTunnel | `X` | C→S | tunnel_name |
| ReqDataConn | `Q` | S→C | （空）请求客户端补充数据连接 |
| NewDataConn | `W` | C→S | session_id, auth_digest, timestamp —— 新数据连接首帧鉴权 |
| StartDataConn | `S` | S→C | tunnel_name, src_addr, src_port, dst_addr, dst_port —— 数据连接路由头 |
| Ping / Pong | `G`/`g` | 双向 | auth_digest, timestamp / error |
| UdpPacket | `D` | 双向 | content(base64), local_addr, remote_addr —— UDP 隧道控制面载荷 |
| KickOut | `E` | S→C | reason |

### 7.3 鉴权（对齐 orbien `core/src/auth`）

- `auth_digest = hex(HMAC-SHA256(token, timestamp秒字符串))`
- 校验：时间窗（`AUTH_SKEW_SECS`）内 + digest 首次使用（ReplayCache 防重放）
- token 为空 = 关闭鉴权（仅限本地调试）

### 7.4 数据连接池与公网接入（对齐 orbien server 流程）

```
公网用户 ──► 节点 :remote_port (TcpTunnel accept)
                │ 1. 从该 client 的数据连接池 pop
                │ 2. 池空 → 经控制连接发 ReqDataConn，等客户端
                │    NewDataConn 补充（超时 10s）
                │ 3. 在取出的数据连接上写 StartDataConn{tunnel_name, src…}
                ▼
           客户端按 tunnel_name 找到隧道 → 连接 local_addr
                → ingress.stream 与数据连接双向拷贝（带宽限速可选）
```

- 数据连接池按 **client session** 组织；Login 携带 `pool_count` 预建数量
- 隧道注册：名称注册表去重 + 端口表（claim/release），失败回滚（对齐 orbien `register.rs`）
- TCP 调优：nodelay + keepalive(30s/10s)；ingress 支持 Proxy-Protocol / XFF（orbien `net/`）

### 7.5 传输层支持矩阵（规划，按序实现）

| 传输 | 复用 | TLS | 阶段 |
|---|---|---|---|
| TCP | yamux（可关） | rustls（自签 rcgen / 证书文件 / mTLS） | **P2 数据面升级目标** |
| QUIC (quinn) | 原生多流 | 内建 | P3 |
| WebSocket | yamux | 依存宿主 TLS | P3 |
| KCP | yamux | 外层 TLS | P4（抗丢包场景） |

ALPN 约定、自签证书生成（rcgen）等细节照 orbien `core/src/transport/tls.rs` 规格。

### 7.6 orbien 底层集成路线（Phase 3 核心）

orbien 仓库分 `core` / `client` / `server` 三个 crate，可复用性不同：

| 部件 | orbien 形态 | gostc-rs 集成方式 |
|---|---|---|
| `orbien-core`（协议+传输+配置） | 独立 lib | **作为库依赖直接引入**（vendored 到 `third-party/orbien`，Apache-2.0 保留声明） |
| `orbien` client | lib + bin（`ClientHandle`/`StartOptions`/`ClientConfig`/`Service`/`reload`/`local_control`） | **库级嵌入** `gostc-rs-tunnel-client`：面板下发的隧道列表生成 `ClientConfig.tunnels`，经 `reload` 热更新，无需重启进程 |
| `orbien-server` | 仅 bin（无 lib） | `gostc-rs-tunnel-server` **子进程托管**：写 `orbien-server.toml`（listen/auth/transport/quicPort/kcpPort）→ 启动/重启子进程；节点传输协议来自面板 |

配置映射（面板 → orbien）：

- 节点 `transport` → server 端 `listen/quicPort/kcpPort` + client 端 `[transport] protocol`
- 隧道 `{name, type, local_addr, remote_port, domain}` → client 端 `[[tunnels]] {name, protocol, service, remotePort, domains}`
- 节点 `secret` / 客户端 `token` → `[auth] type="token" token=...`
- 后续：`[tunnels.transport] bandwidth`（限速）、`basicAuthUser/Password`、`headers`（HTTP 隧道增强）

### 7.7 gostc 功能差距清单（Roadmap 对照 gostc-open）

| gostc 功能 | 状态 | 计划 |
|---|---|---|
| 节点：创建/删除/命令展示/心跳在线 | ✅ | — |
| 节点：传输协议选择（tcp/quic/ws/wss/kcp） | ✅ 配置链路 | orbien 接入后生效 |
| 客户端：创建/命令/在线状态 | ✅ | — |
| 隧道：TCP 转发 | ✅ MVP | 切 orbien 协议（§7.6） |
| 隧道：UDP / HTTP / HTTPS 转发 | ❌ 仅可建 | Phase 3/4（orbien tunnel 类型） |
| 隧道：带宽限制 / Basic Auth / 自定义 Header | ❌ | Phase 4 |
| 端口转发（forward） / 域名解析（host） | ❌ | Phase 4 |
| SOCKS5 代理隧道 | ❌ | Phase 4+（orbien client plugin） |
| P2P 隧道（vKey + 访客，使用方式对齐 gostc） | ✅ 管理面 | 数据面 Phase 3/4（§7.8，原理参考 EasyTier） |
| 秘密隧道 STCP/SUDP（双方客户端 + sk + visitor） | ✅ 管理面 | 数据面 Phase 4（§7.9，自研） |
| 用户：流量配额 / 带宽限制生效 | ❌ 表结构已有 | Phase 4 |
| 流量统计 / 报表 | ❌ | Phase 4（metrics 对齐 orbien counter 栈） |
| 全站统计趋势图 | 简版卡片 | Phase 4 |
| 通知公告 / 系统配置页 | ✅ 基础版 | 邮件通知等 Phase 4+ |
| 操作审计日志 | ❌ 表结构已有 | Phase 4+ |
| ACME 证书自动申请 | ❌ | Phase 4（orbien 内建） |

### 7.8 P2P 隧道自研方案（使用方式对齐 gostc，实现原理参考 EasyTier）

**核查结论（2026-09-29）**：orbien 无 P2P 能力（隧道类型仅 Tcp/Http/Https/Udp，
无 STUN/打洞/直连协调代码）；gostc-open 的 P2P 隧道是「服务方客户端注册节点+内网目标，
访问方凭 vKey 在自己的客户端开访客入口」的使用模型（frp xtcp 语义），其数据面同样
不在开源范围。gostc-rs 自研：**使用方式对齐 gostc，实现原理参考 EasyTier**
（github.com/EasyTier/EasyTier）。

**EasyTier 原理参考要点**：

- NAT 类型探测基于 **STUN**（CLI 暴露 nat_type 列，如 FullCone），辅以路由器
  端口映射（UPnP/NAT-PMP 类）
- **UDP 打洞**支持 NAT4-NAT4 多层嵌套与 IPv6；打洞成功后节点直连（cost=p2p）
- 打洞失败自动经**共享节点中继**，按延迟优先自动选路（直连/中继自动切换）
- 高丢包环境用 KCP/QUIC 代理优化；组网密钥鉴权 + AES-GCM/WireGuard 加密

**gostc-rs 方案**：

1. **使用方式（对齐 gostc，管理面已落地）**：
   - 服务方：创建 P2P 隧道（`tunnels.type='p2p'`：节点 + 内网目标 + vKey）
   - 访问方：凭 vKey 在自己的客户端开访客入口（`tunnel_visitors`：本地监听端口）
   - 双方客户端都只连中央控制台（复用现有连接模型），30s 心跳内自动下发生效
   - 面板：「P2P隧道」页（创建/卡片/访客管理 Modal），不占节点公网端口
2. **协调面**：控制台充当 Rendezvous——按隧道/访客配置交换双方公网映射地址
   与 NAT 类型结果
3. **NAT 穿透（参考 EasyTier）**：STUN 探测（RFC 8489，公网 STUN 服务器可配置）
   分类 NAT——Full/Restricted-cone 可打洞，Symmetric 直接走中继；UDP 打洞由双方
   client 在协调下互发探测包；辅以 UPnP/NAT-PMP 端口映射；双方均有公网 IPv6 时
   优先 v6 直连（跳过打洞）
4. **数据面**：直连成功后跑 yamux + orbien 消息帧（§7.2/§7.3）；失败回退节点
   中继（服务方隧道所选节点）；面板标记连接模式（直连/中继），延迟优先自动选路
5. **安全**：vKey 与用户密码同一套加盐 KDF 哈希存储；访客连接时 verify 校验；
   数据面加密沿用 orbien 传输层（rustls）

**实现状态**：管理面 ✅（p2p 类型 + 访客 API + 面板页）；协调面 ✅（`POST /p2p/sessions`
内存 Rendezvous：client token 鉴权、按面板信任模型派生 service/visitor 角色、注册即
返回对端快照、10 分钟 TTL，控制台重启后客户端自动重新注册）；STUN 客户端 ✅
（tunnel-client `stun.rs`：RFC 5389/8489 Binding 编解码、XOR-MAPPED-ADDRESS v4/v6
解析、双服务器比对判定 Symmetric NAT，codec 单测覆盖；Phase 4 接入打洞）。UDP 打洞
与直连数据面（yamux + orbien 帧）Phase 4——先落地中继路径，再攻打洞直连。

排期：与 §7.9 秘密隧道共用访客与协调设施；数据面随 Phase 3/4 接入。

### 7.9 秘密隧道（STCP/SUDP）自研方案（对齐 frp，orbien/gostc-open 均无此能力）

**核查结论（2026-09-29）**：frp 的 STCP/SUDP（服务方与访问方**双方都运行客户端**、
访问方以 visitor + sk 密钥接入、无公网端口暴露）在 gostc-open 与 orbien 中均不存在
——gostc-open 的「私有隧道」是节点转发模型，orbien 隧道仅 Tcp/Http/Https/Udp。
gostc-rs 自研，数据模型与管理面已落地，数据面随 Phase 3/4 接入：

1. **模型**：`tunnels.type ∈ {stcp, sudp}`（服务方隧道，`sk_hash` 存密钥哈希，
   不存明文、不下发 UI）+ `tunnel_visitors`（访问方入口：visitor 客户端 id +
   本地监听端口，唯一约束 tunnel×client）。管理面 API：
   `GET/POST /tunnels/:id/visitors`、`DELETE /tunnels/:id/visitors/:vid`。
2. **鉴权**：sk 用与用户密码同一套加盐 KDF（auth.rs `hash_password`）哈希；
   访客连接时访问方客户端提交 sk，服务方客户端以 `verify_password` 校验通过后
   才建立转发（ReplayCache 防重放，复用 orbien §7.3）。
3. **协调面**：全部复用现有中央控制台连接模型——服务方与访问方 client 都只连
   控制台；控制台按 `tunnel_visitors` 下发 visitor 配置（对端身份、sk 哈希、
   目标隧道），30 秒心跳内自动生效，与普通隧道一致。
4. **数据面路径**：优先 P2P 直连（复用 §7.8：STUN 打洞，双方 client 直连）；
   打洞失败回退节点中继（服务方隧道所选节点）。sudp 走同模型，载荷为 UDP。
5. **面板**：「秘密隧道」页已就位（stcp/sudp 创建 + sk 设置 + 访客管理 Modal）；
   卡片不显示公网地址，标注「秘密 (sk)」。

排期：管理面 ✅（本轮）；数据面 Phase 4（先于/随 P2P 直连通道一起落地，回退路径
可先走节点中继提前可用）。

---

## 8. 部署形态

### 8.1 二进制直跑（当前主推）

```
[公网 IP:8080]  gostc-rs-admin            # 中央控制台（默认 admin/admin）
[公网 IP:7502]  gostc-rs-tunnel-server    # 节点（面板创建后给命令）
[内网机器]      gostc-rs-tunnel-client    # 客户端（面板创建后给命令）
```

节点与客户端启动不分先后；均由面板下发的隧道配置驱动。

### 8.2 Docker（Phase 5）

```yaml
services:
  admin:
    image: david88558855/gostc-rs-admin:latest
    network_mode: host
    volumes: [./data/admin:/data]
  node:
    image: david88558855/gostc-rs-tunnel-server:latest
    network_mode: host
    depends_on: [admin]
  client:                                  # 部署在内网机器
    image: david88558855/gostc-rs-tunnel-client:latest
    network_mode: host
    command: --api http://admin.example.com:8080 --token <token>
```

---

## 9. 实施阶段

### Phase 1：基础设施 ✅

- [x] Cargo workspace 骨架 + GitHub Actions 三平台编译/clippy
- [x] `common` 共享 crate；`admin` axum 框架、配置自动生成、健康检查

### Phase 2：管理后端 MVP ✅

- [x] 用户/节点/客户端/隧道 CRUD + JWT + argon2
- [x] 节点注册心跳（`/nodes/register`，JOIN 下发隧道配置）
- [x] 客户端连接（`/clients/connect`，无启动顺序）
- [x] 内嵌中文 Web 面板（版块对齐 gostc：全站统计/节点管理/客户端/私有隧道/用户管理/关于）

### Phase 3：隧道数据面（进行中）

- [x] TCP 隧道 MVP（简化协议：`GOSTC1` 握手 + `DIAL` 借道）
- [x] 节点传输协议选择（tcp/quic/websocket/kcp）：迁移 004 + API + 面板下拉/编辑，注册与客户端连接响应均已下发
- [ ] **orbien 底层接入（§7.6）**：vendored `orbien-core`；client 库级嵌入（ClientConfig + reload 热更新）；server 子进程托管
- [ ] 面板隧道配置 → orbien `ClientConfig.tunnels` 映射（name/protocol/service/remotePort/domains）
- [ ] HMAC-SHA256 鉴权 + 防重放替代明文 token
- [ ] TCP + yamux 复用 + rustls（自签证书）；QUIC/WebSocket/KCP 按节点协议生效
- [ ] UDP 隧道转发（UdpPacket 控制面载荷）
- [ ] 带宽限制（token bucket，按隧道配置）

### Phase 4：HTTP/HTTPS 与增强

- [ ] 节点内置 HTTP/HTTPS 域名路由（含 TLS 终止/透明转发两种模式，对齐 orbien）
- [ ] SOCKS5 客户端插件
- [ ] QUIC / WebSocket / KCP 传输
- [ ] 流量统计与配额（traffic_logs + 限速）

### Phase 5：发布与运营

- [ ] tag 触发多平台 Release；Docker 镜像；文档站

---

## 10. 许可证与归属

Apache-2.0。`NOTICE` 声明：gostc-open（管理面设计参照）、orbien-org/orbien（数据面协议参照）、rathole、frp（设计参考）。未复制上游源码，协议为规格级对齐（自研实现）。

---

## 11. 风险与开放问题

| 风险 / 问题 | 影响 | 缓解 |
|---|---|---|
| orbien 协议无正式规范文档 | 规格对齐依赖源码阅读 | 以 §7 记录快照；实现时对照 orbien v3.8 源码逐条验证 |
| 数据连接池在节点重启后需重建 | 客户端短暂不可达 | 客户端 worker 循环自动重连（已有 backoff 机制） |
| yamux/quinn 引入增加二进制体积 | 偏离 ≤10MB 目标 | 按功能开关（feature flag）裁剪 |
| SQLite 单写者 | 多 admin 实例不可行 | 明确单实例部署；多实例属非目标 |
| gostc-open web 为 Vue 工程 | 面板对齐以单文件 HTML 复刻 | 已完成主体版块；细节渐进补齐 |

---

## 12. 版本记录

- **v0.7 — 2026-09-29**：P2P 隧道管理面落地（使用方式对齐 gostc）——迁移 007 新增 p2p 隧道类型（vKey 复用 sk_hash、访客复用 tunnel_visitors）；面板 P2P 页改为 gostc 式可创建（服务方节点+内网目标+vKey，访客管理）；dashboard 统计真实 p2p 数；§7.8 重写：实现原理参考 EasyTier（STUN NAT 探测 + UDP 打洞 NAT4-NAT4 + UPnP/NAT-PMP + 中继回退 + 延迟优先选路），数据面排期 Phase 3/4

- **v0.6 — 2026-09-29**：自研秘密隧道 STCP/SUDP（核查确认 gostc-open 与 orbien 均无 frp visitor 模型，§7.9）——管理面落地：tunnels 类型扩展（stcp/sudp + sk_hash 加盐哈希）、tunnel_visitors 访客表与 API（GET/POST/DELETE）、面板新增「秘密隧道」页（sk 创建 + 访客管理 Modal，无公网端口暴露）；数据面路径复用 §7.8 P2P 直连 + 节点中继回退，随 Phase 4 接入
- **v0.5 — 2026-09-29**：控制台对齐 gostc-open——面板重写为 12 项完整菜单（全站统计/系统配置/通知公告/用户管理/节点管理/客户端/域名解析/端口转发/私有隧道/代理隧道/P2P隧道/关于），节点与客户端改 gostc 式卡片网格（域名解析/端口转发/P2P 功能开关与 tabs、连接协议含 WSS 五选项）、五类隧道页共用卡片骨架（状态开关/更多操作/访问密钥）、用户表格页加编辑、全站统计 10 卡 + 流量排行卡；后端补 dashboard 聚合 / notices / settings API 与 nodes 扩展列；§7.8 落档 P2P 自研方案（核查确认 orbien 无 P2P/STUN 能力，需自研：STUN 打洞 + 中继回退）
- **v0.4 — 2026-09-29**：明确产品定位——gostc 为产品蓝本（Rust 重构），orbien 为**直接复用的数据面底层**（client 库级嵌入 + server 子进程托管，§7.6）；新增节点传输协议选择（tcp/quic/websocket/kcp，§7.0，配置链路已实现）；新增 gostc 功能差距清单（§7.7）
- **v0.3 — 2026-09-29**：确立双对齐基线（管理面 gostc / 数据面 orbien-org/orbien）；§7 落档 orbien 协议规格快照（消息帧/类型表/HMAC 鉴权/数据连接池/传输矩阵）；移除 gateway 组件（HTTP/HTTPS 改由节点内置路由）；更新仓库结构、部署形态与 Phase 进度
- v0.2 — 2026-09-29：传输层基线由 rathole 调整为 orbien
- v0.1 — 初始提案
