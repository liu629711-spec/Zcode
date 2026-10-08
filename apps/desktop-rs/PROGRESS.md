# Rust 重构进度盘点

盘点时间：2026-10-08 17:25
分支：`product/rust-frontend`（自 `product/first-batch` 切出，工作树干净）
提交：23 个 commit，**全部未 push**

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

### 3. 存储层
- `rusqlite`(bundled) 直读 `~/.zcode/v2/tasks-index.sqlite`
- 分区查询 SQL 照抄真源（pinned/archived/active 互斥）
- pinned 置顶、archived 归档、unarchive 取消归档、软删除通道全通

### 4. UI 组件（已迁 9 块）
侧栏骨架 / 新任务按钮 / 会话行 / 五视图切换 / **置顶区** / **归档区** /
命令中心(⌘K) / 树形文件树 + 代码预览 / 顶栏 / 对话区（markdown + 行视图 + toolCall 卡）
+ **分组区拖拽重排（view/join/落位/状态机/DOM 接线/落库/视图接线 七块）**
+ **toast 提示**（真源 toast.tsx 核心契约，4 位置 + 4 变体）

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
1. 侧栏 dnd + 任务分组（`workspace-grouped-tasks/` 4307 行，自定义组 + 员工 roster）
   - [x] **纯逻辑层已迁**：`groupedTasks/view.rs` 1:1 翻译 `view.ts`(581) + `ids.ts`，
     真源 16 个导出函数全覆盖 + 13 个内部 helper，24 个单测。
   - [x] **join 层已迁**：`groupedTasks/join.rs`，task_ids → 完整 task（含
     workspaceIdentity），`grouped.rs` 已改走该链路。
   - [x] **拖拽落位决策已迁**：`groupedTasks/dnd.rs`，6 个 preview 函数 +
     视图签名 + 7 层分发链，23 个单测。
   - [x] **事件适配层已迁**：`groupedTasks/dragRuntime.rs`（状态机）+
     `groupedTasks/domEvents.rs`（HTML5 DnD 接线），方案 = 原生 draggable。
   - [x] **顺序落库已迁**：后端 `taskgroup::apply_grouped_order` + Tauri 命令
     `task_group_apply_order`（顶层顺序 + 组内顺序 + 跨组改成员关系）。
   - [x] **已接线到视图组件**（拖拽实际生效）：`joined_view` 信号承载视图，
     `GroupedTaskRow` / `LooseTaskRow` / `GroupItem` 挂 draggable + 事件闭包，
     组头 / 组尾 / 空内容区加 `data-drop-type`，drop 后落库，失败回滚。
   - [x] **标题走马灯已迁**：`taskTitle.rs`，含渐隐 + 滚动常量与溢出判定。
   - [x] **右键菜单已迁并接线**：`groupedTasks/taskMenu.rs`（13 项按能力分三类，
         纯视图变换类可用、其余保留为禁用项不隐藏）+ `grouped.rs` 的
         `TaskContextMenu` 渲染层（遮罩关闭 / 动作执行 / 与拖拽共用落库通道）。
   - [x] **hover 快捷按钮已迁**：移到顶部（可用）/ 关闭（禁用占位）/
         文件树（条件不渲染），与拖拽共用落库通道。
   - [x] **拖拽预览已迁**：setDragImage + 屏幕外预览元素（task 行纯展示版 /
         group 组头复刻），dragend 按 data-drag-preview 清理。
   - [ ] `sticky-group-header.tsx` **暂缓**：真源吸顶依赖虚拟化滚动
         （IntersectionObserver + getBoundingClientRect 测量 + virtualized-*），
         Rust 侧列表无虚拟化且长度尚短，等列表长度成为问题再连虚拟化一起迁。
2. [x] **逐 token 流式已通**：delta 累积（stream.rs，11 测）+ 通知桥接入
     WireAssembler（lib.rs `forward_conversation_frame`）——fragment 先
     staging，收齐校验（base64/字节增量/CRC32/UTF-8/JSON/信封）后才以
     complete 信封放行，前端见不到分片；fault 触发 same-sub resync 走
     恢复阶梯。顺手修掉 checksum 建模 bug（扁平 `checksumValue` →
     嵌套 `{algorithm, value}`，wire.ts:7-12）。

**后续**

3. [x] **toolCall 专属卡已接线**：框架 + 8 renderer 此前已迁但零接线，
     本轮补 toolCallRowAdapter（v4 行→旧形态）/ toolStatus / toolError /
     顶层 ToolCallBlock 编排，ConversationRowView 扩字段，app.rs 换用。
     execute/read/search/todo 出专属卡，edit 待 fileSummaries（文件 chip），
     其余 fallback 兜底（未迁不隐藏）。裁剪：流式半截 JSON 预览、
     workflowRun 联接、CUA 组、入场动画。
   - [x] **fileSummaries 已迁**：`fileSummaries.rs` + `toolDiffPreview.rs`
         （unified diff 构建含阈值回退与多文件守卫）+ types/heuristics 补齐。
         edit 卡文件 chip / diff 计数 / patch 全通。
   - [x] **工作行分组器已迁并接线**：`conversationWorkItems.rs`（含
         `exploreToolCall.rs` 判定层 + regex-lite 依赖）。连续终端命令折叠成
         聚合卡（children 递归渲染），Agent↔subagent 配对抑制。explore 分组
         逻辑就绪暂关（explore.tsx 490 行未迁）。
   - [x] **explore.tsx 已迁并接线**：`renderers/explore.rs`——归桶分类
         （搜索/列表/文件）、父摘要拼接、折叠态实时子摘要（read chip /
         search 文本 / shell 命令 / todo 进度）、递归展开区。explore 分组
         开关已翻 true（真源默认），三个分组的开关状态与真源常量全面对齐。
   - [ ] toolCall 剩余：其余 ~50 个 renderer（按需）、CUA 组
         （conversationCuaGroups 281 行）、工作流 run 联接、
         agent.tsx（Agent 专属卡 + subagent 卡）、ToolSnapshotFieldNotice。
4. 文件树剩余项：git 状态 / fileWatcher / 虚拟滚动 / 拖拽右键菜单
5. 归档区剩余：变更统计（changeSummary）、清空全部归档按钮
6. 富文本输入（`LexicalChatInput` 1535 行 → Rust 富文本方案待定）
7. 设置页 / 资产库 / onboarding 等独立大区（55 万+6 万行）

**待决**

8. **push 43 个 commit**（等你发话）

---

## 六、验证现状

- 前端 **386 测试全绿**（文件树 flatten/compact、图标解析、置顶区、归档区、
  流式重组、**分组视图重排 24**、**join 层 11**、**拖拽落位 23**、
  **拖拽运行时 12**、**DOM 接线与落库载荷 7**、**拖拽端到端 2**、**toast 9**、**标题走马灯 9**、**右键菜单 9**）
- 后端：taskgroup.rs 新增 7 个落库单测；后端 lib 测试**在本机无法运行**
  （见第七节 2），故另用独立 crate + 真实 SQLite 跑了 8 组落库行为验证，全通过
- 后端原有 23 测试（协议信封 4 + agent 三件套 6 + taskdb 12）+ 4 个 live 测试
  （待环境修复后运行）
- trunk 0 错误 0 警告

---

## 七、本机环境坑（Windows + GNU 工具链）

### 1. linker 必须显式指定 WinLibs（已修）

PATH 里 **llvm-mingw 排在 WinLibs 前面**，`x86_64-w64-mingw32-gcc` 会被解析到
llvm-mingw 的 clang wrapper，它找不到 libgcc，导致**任何** build script 链接失败：

```
lld: error: unable to find library -lgcc
lld: error: unable to find library -lgcc_eh
```

已生成 `apps/desktop-rs/.cargo/config.toml` 写死 WinLibs 的 gcc/ar。
该文件含机器相关绝对路径，**已 gitignore**，各机需自行生成（模板见文件内注释）。

### 2. 后端 lib 测试在本机跑不起来（预先存在，非本轮引入）

`cargo test -p zcode-desktop-rs --lib`链接阶段报：

```
export ordinal too large: 99619
```

原因：`src-tauri` 的 `crate-type = ["staticlib","cdylib","rlib"]`，
Tauri 的 Windows cdylib 导出符号量超过 MinGW ld 的 65535 ordinal 上限。
加 `--lib`（只取 rlib 测试目标）可绕过链接；但**运行时仍崩**：

```
exit code: 0xc0000139, STATUS_ENTRYPOINT_NOT_FOUND
```

诊断为 `WebView2Loader.dll` 静态导入 +本机版本不匹配，进程加载即失败。

已用 `git stash` 验证：移除本轮全部改动后同样崩溃，**与重构无关**，
属环境问题。待决：本机装匹配的 WebView2Loader runtime，或改用 msvc 工具链。