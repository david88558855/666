# gostc-rs

> 用 Rust 重写的内网穿透管理平台。功能对照 [SianHH/gostc-open](https://github.com/SianHH/gostc-open)。

**当前进度：Phase 1-2 已完成 —— `gostc-rs-admin` 管理后端（REST API + 内嵌 Web 控制面板）可用，数据面（隧道转发）开发中。**

## 快速开始

从 GitHub Actions Artifacts 下载对应平台的压缩包（Linux/macOS 为 `.tar.gz`，Windows 为 `.zip`），解压后运行：

```bash
tar -xzf gostc-rs-x86_64-unknown-linux-gnu.tar.gz
chmod +x gostc-rs-admin
./gostc-rs-admin
```

- 首次运行会在当前目录自动生成 `config.toml`（含随机 `jwt_secret` 和随机管理员密码），并在控制台打印登录信息
- 浏览器打开 `http://127.0.0.1:8080/` 进入 Web 控制面板（登录后请立即修改密码）
- 默认端口 `8080`，可在 `config.toml` 的 `server.bind_addr` 中修改；也可用环境变量 `GOSTC_RS_CONFIG` 指定其他配置路径

## 使用流程（与 gostc 一致，全程 Web 配置）

1. **建节点**：面板「节点」页新建节点（隧道端点填客户端要连的地址，如 `节点公网IP:7502`）→ 面板直接给出节点运行命令，复制到**有公网 IP 的服务器**上执行：
   ```bash
   ./gostc-rs-tunnel-server --api http://面板地址:8080 --secret <节点Secret>
   ```
2. **建客户端**：面板「客户端」页新建客户端（选择刚建的节点）→ 面板给出客户端运行命令，复制到**内网机器**上执行：
   ```bash
   ./gostc-rs-tunnel-client --server <节点IP>:7502 --token <客户端Token>
   ```
3. **建隧道**：面板「隧道」页新建隧道（选客户端、类型、本地地址如 `127.0.0.1:22`、远程端口如 `6022`）并启用 → **自动下发**，节点自动开放公网端口，客户端自动连接本地服务，无需重启任何进程。

之后访问 `节点IP:6022` 即等于访问内网机器的 `127.0.0.1:22`。

当前限制：TCP 隧道可用；UDP / HTTP / HTTPS 隧道已在面板支持创建，转发将在后续版本实现。

## 项目入口

- 完整设计：[ARCHITECTURE.md](./ARCHITECTURE.md)
- 许可证：[LICENSE](./LICENSE)
- 归属声明：[NOTICE](./NOTICE)

## 组件预览

仓库采用 Cargo workspace，包含 4 个二进制：

| Crate | 作用 |
|---|---|
| `gostc-rs-admin` | 管理后端：REST API + 控制面 WS + 静态 web 托管 |
| `gostc-rs-tunnel-server` | 隧道服务端：监听客户端反向连接、暴露公网端口 |
| `gostc-rs-tunnel-client` | 隧道客户端：部署在内网，反向打洞暴露内网端口 |
| `gostc-rs-gateway` | 自定义域名网关：SNI 路由 + TLS 终止 |

## 构建

CI 在 GitHub Actions 中完成（不在本机编译）。详见 `.github/workflows/rust-build.yml`。

如需本地构建：

```bash
rustup install stable
cargo build --workspace --release
```

## 许可

Apache-2.0，详见 [LICENSE](./LICENSE)。