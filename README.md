# gostc-rs

> 用 Rust 重写的内网穿透管理平台。功能对照 [SianHH/gostc-open](https://github.com/SianHH/gostc-open)。

**当前阶段：架构方案评审中，暂未进入代码实施。**

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