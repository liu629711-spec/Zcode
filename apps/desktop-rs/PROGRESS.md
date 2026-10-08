# Rust 重构进度盘点

盘点时间：2026-10-08 17:10
分支：`product/rust-frontend`（自 `product/first-batch` 切出，工作树干净）
提交：22 个 commit，**全部未 push**

---

## 一、方向（不变）

全量 Rust 重构桌面端，**Tauri 2 + Leptos 0.8**，废弃 Electron/React。
唯一红线：**布局/交互/视觉 1:1 照抄 React 真源**，Leptos 只是替换实现语言，不重新设计。

样式不能自造近似值——真源类名在 shadcn 语义体系里（`accent` = hover 底色，不是品牌色），
必须以 `packages/ui/src/styles.css` 为准。

---

## 二、规模对比（说明真实进度）

| | 真源 React | Rust 侧 |
|---|---|---|
| `packages/ui/src` | 405,640 行 / 1,885 文件 | — |
| `apps/desktop-rs` 全部 | — | 4,913 行 |

**表面看 2%。但这个数字严重失真**，原因：

真源 40 万行里绝大部分**不是桌面端主链**：

| 目录 | 行数 | 是否本次要迁 |
|---|---|---|
| `asset-library/` | 60,614 | 否（独立功能区，未接主链） |
| `v4/` | 56,108 | **部分是**（已迁 ConversationRowView 数据模型 + 行视图） |
| `settings/` | 55,115 | 否（设置页独立） |
| `lib/` | 31,485 | 部分（已迁 fileDisplayHelpers 图标映射） |
| `app-shell/` | 16,534 | **是**（已迁 WorkspaceShellLayout 骨架） |
| `workspace-file-tree/` | 3,498 | **是**（本轮已完成） |
| 其余分散模块 | — | 逐块推进 |

按"桌面端主链 + 已点名的功能块"算，实际待迁约 **8-10 万行**，
已迁约 **1 万行**（含协议层、agent 运行时、存储层）→ **真实进度约 10%**。

---

## 三、已完成的四层

### 1. 基础设施（完成）
- Tauri 2 + Leptos CSR(wasm32) + trunk 打包，工具链全通
- 主题真源直引 `packages/ui/src/styles.css`（834KB，React 类名全集进编译）
- 图标 1:1：material-icons 1146 个 SVG + 脚本生成的图标名映射表
- 代码高亮（highlight.js 11.11.1，经 esbuild 打包）

### 2. Agent 运行时与协议桥（完成，真链路已验证）
- `zcode-protocol` 信封层翻译，80 个方法名常量
- spawn 真实 `node zcode.cjs app-server --stdio`，capabilities 握手通过
- **v4 主路径打通**：`v4/conversation/rowsRange` 数据源 + `v4/command`（createSession/sendText）
- 断线自动重连、进程树清理（taskkill /T /F）

### 3. 存储层（本轮新增）
- `rusqlite`(bundled) 直读 `~/.zcode/v2/tasks-index.sqlite`
- 分区查询 SQL 照抄真源（pinned/archived/active 互斥）
- pinned 置顶、archived 归档通道已通

### 4. UI 组件（已迁 8 块）
侧栏骨架 / 新任务按钮 / 会话行 / 五视图切换 / 置顶区 / 命令中心(⌘K) /
树形文件树 + 代码预览 / 顶栏 / 对话区（markdown + 行视图 + toolCall 卡）

---

## 四、方向上的三个关键认知

1. **置顶/归档这类功能不在 agent 协议里**，真源存在 node 侧 SQLite。
   Rust 版直接自己实现存储层（`taskdb.rs`），不绕路。

2. **跨界对齐必须以真实数据探测为准，不能靠函数命名猜语义。**
   差点白做：`getWorkspaceHash` 名字像主键，实际是原始路径。

3. **生成器 + 独立复核器** 这套办法有效：图标映射表由脚本从真源生成，
   另写复核脚本独立复刻真源逻辑对拍，抓出了真 bug。

---

## 五、剩余工作（按优先级）

**下一批**
1. 归档区 UI（数据通道已通，照 `WorkspaceArchivedTasksSection` 翻译）
2. 侧栏 dnd + 任务分组（`workspace-grouped-tasks/` 4307 行，自定义组 + 员工 roster）
3. 逐 token 流式（v4 gateway 订阅 + wire reassembly，当前是准流式）

**后续**

4. toolCall 专属卡（`ToolCallBlocks/` 15345 行，按 execute/read/edit 分流）
5. 文件树剩余项：git 状态 / fileWatcher / 虚拟滚动 / 拖拽右键菜单
6. 富文本输入（`LexicalChatInput` 1535 行 → Rust 富文本方案待定）
7. 设置页 / 资产库 / onboarding 等独立大区（55万+6 万行）

**待决**

8. **push 22 个 commit**（等你发话）

---

## 六、验证现状

- 后端 18 测试（协议信封 4 + agent 三件套 6 + taskdb 8）+ 3 个 live 测试（真实 agent、真实 SQLite 库）
- 前端 20 测试（文件树 flatten/compact、图标解析、置顶区）
- trunk 0 错误 0 警告