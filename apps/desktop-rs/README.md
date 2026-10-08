# ZCode 桌面端（Rust 重构）

Tauri 2 + Leptos 0.8（WASM）重构的桌面端壳。目标：替换 Electron（双份 Chromium + Node main）
为系统 WebView2 + Rust 主进程，降低运行时占用；业务逻辑逐步 Rust 化。

## 目录结构

```
apps/desktop-rs/
├── Cargo.toml           # cargo workspace（frontend + src-tauri）
├── frontend/            # Leptos CSR 前端（wasm32-unknown-unknown）
│   ├── index.html       # trunk 入口
│   ├── Trunk.toml
│   ├── styles.css
│   └── src/
│       ├── main.rs      # 挂载入口
│       └── app.rs       # 应用壳（侧边导航 + 主区 + IPC 自检）
└── src-tauri/           # Tauri 主进程（Windows: WebView2）
    ├── tauri.conf.json
    ├── capabilities/default.json
    ├── icons/           # 由 packages/desktop/build/icon.png 生成
    └── src/
        ├── main.rs
        └── lib.rs       # run() + tauri command
```

## 启动

前置：rustup（stable）、`rustup target add wasm32-unknown-unknown`、trunk、tauri-cli（本机均已安装）。

```bash
cd apps/desktop-rs
cargo tauri dev     # 起 trunk serve(127.0.0.1:1420) + 桌面窗口，热更新
```

窗口内点「检查主进程连接」验证 IPC（前端 WASM -> 主进程 command `get_app_info`）。

> 注意：`tauri.conf.json` 的 `beforeDevCommand` 以 `apps/desktop-rs` 为工作目录，
> 请从该目录运行命令。

## 阶段计划

1. **骨架（当前）**：窗口 + 布局壳 + IPC 链路自检。
2. **协议对接**：主进程经 stdio 按 `packages/shared/src/zcode-protocol` 对接现有
   Agent 运行时；前端经 Tauri event/command 收发消息。
3. **核心视图**：会话时间线、消息流（流式 markdown + 代码高亮）、终端、任务看板。
4. **能力迁移**：按 `packages/ui` 的视图逐个迁移；高亮/预览等重组件可走
   WASM（项目已有先例：xlsx 渲染即 wasm）。

## 与旧版（Electron）的关系

- 旧桌面端（`packages/desktop`）保持不动，双版本并行，直到新版功能对齐。
- Web 端（`packages/web` + `packages/ui`）不受影响，继续走 React 方案。
- 协议层（`packages/shared`）为双方共用事实标准，Rust 端只做客户端实现，不改协议。
