#!/usr/bin/env node
/**
 * 一次性生成器（V3-3 逐字收录备货）：把三个 MIT/CC BY 开源仓库的原版组件源码
 * 转成素材库的资产卡（catalog/assets/*.ts）。
 *
 * 三个货源（用户拍板"直接用他们的，他们开源"）：
 *  - Beautiful UI · github.com/TurboKach/ai-native-react-components（MIT，19 件 AI 界面原语）
 *  - RareUI      · github.com/Codewithswappy/RareUI（MIT，26 件动画组件）
 *  - UIverse     · github.com/uiverse-io/galaxy（CC BY 4.0 / 仓库 MIT，精选控件片段）
 *
 * 用法：
 *   node scripts/import-open-source-assets.mjs <refs 目录> [输出目录]
 *   refs 目录里放三份浅克隆：
 *     <refs>/ai-native-react-components/  <refs>/rareui/  <refs>/galaxy/
 *   （RareUI 有 5 件只在 public/r/*.json registry 里，先解包到 <refs>/rareui-extracted/）
 *
 * 产物：catalog/assets/{beautifului,rareui,uiverse}-*.ts，并在 stdout 打印 index 注册片段。
 * 只在备货时跑一次，产物随代码提交（与 import-design-systems.mjs 同模式）。
 * 预览产物由 scripts/build-asset-previews.mjs 生成（读卡片里的 preview 字段）。
 */

import fs from "node:fs";
import path from "node:path";

const refsRoot = process.argv[2];
const outDir = process.argv[3] ?? "packages/ui/src/asset-library/catalog/assets";
if (!refsRoot || !fs.existsSync(refsRoot)) {
  console.error("用法：node scripts/import-open-source-assets.mjs <refs 目录> [输出目录]");
  process.exit(1);
}
fs.mkdirSync(outDir, { recursive: true });

const BEAUTIFULUI = path.join(refsRoot, "ai-native-react-components");
const RAREUI = path.join(refsRoot, "rareui");
const RAREUI_EX = path.join(refsRoot, "rareui-extracted");
const GALAXY = path.join(refsRoot, "galaxy");

/** 逐字读一个源码文件（保留原样，含上游注释）。 */
function readSource(file) {
  return fs.readFileSync(file, "utf8");
}

/** RareUI 的 registry-only 件（public/r/*.json 里的 content 逐字）。 */
function readRegistry(component) {
  const raw = JSON.parse(fs.readFileSync(path.join(RAREUI, "public/r", `${component}.json`), "utf8"));
  return raw.files.map((file) => ({ name: path.basename(file.path), content: file.content }));
}

const written = [];
/** 写一张卡：body 是 AssetManifest 字面量（不含 import/头注释）。 */
function writeCard(fileName, id, header, body) {
  const varName = `${assetVarName(id)}Asset`;
  const file = `import type { AssetManifest } from "../types.js";

${header}
export const ${varName}: AssetManifest = ${body};
`;
  fs.writeFileSync(path.join(outDir, fileName), file, "utf8");
  written.push({ file: fileName, varName, id });
}

/** kebab-id → PascalCase 变量名：beautifului-loading-state → BeautifulUiLoadingState。 */
function assetVarName(id) {
  return id
    .split("-")
    .map((part) => {
      if (part === "beautifului") return "BeautifulUi";
      if (part === "rareui") return "RareUi";
      if (part === "uiverse") return "UIverse";
      if (part === "3d" || part === "d") return part.toUpperCase();
      return part.charAt(0).toUpperCase() + part.slice(1);
    })
    .join("");
}

/**
 * 逐字收录的图纸要在文件头带上版权与许可声明。
 *
 * 上游仓库的 LICENSE 只在仓库根一份，源码文件自身没有版权头（Beautiful UI / RareUI
 * 都是如此；UIverse 片段仅在 <style> 里带 "From Uiverse.io by <作者>" 一行）。
 * MIT 与 CC BY 4.0 都要求分发时随附声明，所以这里在源码**原文之上**前置一段声明块——
 * 原文一字不动，声明另起注释，两者都在同一份交付文件里。
 *
 * 注释语法按图纸语言选：tsx/ts 用包星注释，html 用 HTML 注释（HTML 里塞 JS 注释会被
 * 当成正文渲染出来）。
 */
function licenseHeader({ title, file, author, copyright, license, repo, sourceUrl, extra }, syntax = "js") {
  const lines = [
    `${title}`,
    ``,
    `来源：${repo} · ${file}`,
    `原址：${sourceUrl}`,
    `作者：${author}`,
    `版权：${copyright}`,
    `许可：${license}`,
    ...(extra?.length ? [``, ...extra] : []),
    ``,
    `以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。`,
  ];
  if (syntax === "html") {
    return [`<!--`, ...lines.map((line) => `  ${line}`.trimEnd()), `-->`, ``].join("\n");
  }
  return [`/*!`, ...lines.map((line) => ` * ${line}`.trimEnd()), ` */`, ``].join("\n");
}

/** TS 字面量序列化（JSON.stringify 保证转义安全；字符串里的换行会变 \n，运行时还原）。 */
const ts = (value) => JSON.stringify(value);

/**
 * 预览离线化用的内联占位图（沙箱禁外链，上游硬编码的远程图片在预览副本里换掉）。
 * 都用 base64，避免任意值类名里的引号问题。
 */
const inlineSvg = (svg) => `data:image/svg+xml;base64,${Buffer.from(svg).toString("base64")}`;
const AVATAR_URI = inlineSvg(
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80"><rect width="80" height="80" rx="40" fill="#c7d2fe"/><circle cx="40" cy="31" r="14" fill="#6366f1"/><path d="M12 76c4-17 15-25 28-25s24 8 28 25z" fill="#6366f1"/></svg>`,
);
const PHOTO_URI = inlineSvg(
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 300"><rect width="200" height="300" fill="#e2e8f0"/><circle cx="100" cy="112" r="40" fill="#94a3b8"/><path d="M28 300c8-56 36-84 72-84s64 28 72 84z" fill="#94a3b8"/></svg>`,
);
const NOISE_URI = inlineSvg(
  `<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120"><filter id="n"><feTurbulence type="fractalNoise" baseFrequency="0.85" numOctaves="3"/></filter><rect width="120" height="120" filter="url(#n)" opacity="0.35"/></svg>`,
);

/** 把上游硬编码的远程图片替换成内联占位图（预览专用补丁，图纸原文不动）。 */
function offlineImagePatches(urls, { avatar = false } = {}) {
  return urls.map((url) => ({
    from: url,
    to: avatar ? AVATAR_URI : PHOTO_URI,
    note: "预览离线化：远程图片换内联 data URI（沙箱禁外链）",
  }));
}

// ============================================================
// 1. Beautiful UI（TurboKach/ai-native-react-components，MIT，19 件全收）
// ============================================================
// 上游：每件一个自包含 .tsx（组件本体）+ app/page.tsx 的 COMPONENTS 列表；
// 多数组件无需 props（自带演示数据），selection-actions 还带两个 atoms 依赖。
// 预览主题 = app/globals.css 的 token（scripts/asset-preview-themes/beautifului.css）。

const BEAUTIFULUI_COMPONENTS = [
  { file: "loading-state.tsx", id: "loading-state", title: "加载态（像素网格）", titleEn: "Loading State",
    desc: "长任务的像素网格加载器：雪佛龙波前推进 + 流光文案 + 实时计时。", descEn: "Pixel-grid loader with a shimmering label and a live elapsed timer.",
    category: "block", tags: ["ai-chat", "加载", "等待", "beautifului", "像素"],
    demo: `import Comp from "./loading-state";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「像素网格加载态」装进我的项目：智能体长时间干活时的加载指示器——3×3 像素网格按雪佛龙波前（chevron wavefront）错位点亮循环推进，周期短于横扫周期因而看起来永远有两道波在飞；旁边是流光扫过的文字标签（Shimmer，light sweep 沿字形滑过而不是整块闪）；跟随一个等宽 tabular 数字的实时耗时计时（100ms 一跳，超过 60 秒转 m/s 格式）。三种变体：Drive（方格）/Dots（圆点）/Orbit（彗星绕圈，按 perimeter 顺序点亮）。尊重 prefers-reduced-motion：网格冻结在暗态但计时继续走。先看现有的加载/骨架组件，融入而不是覆盖。" },
  { file: "thinking.tsx", id: "thinking", title: "思考过程（可展开轨迹）", titleEn: "Thinking",
    desc: "可展开的智能体思考轨迹：步骤列表/推理/联网搜索/编码四种形态。", descEn: "Expandable agent trace: steps, reasoning, search and coding variants.",
    category: "block", tags: ["ai-chat", "思考态", "轨迹", "beautifului", "折叠"],
    demo: `import Comp from "./thinking";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「思考过程轨迹」装进我的项目：智能体推理过程的可展开轨迹组件——收起时一行「Thinking」带 spinner，跑完后变「Thought for 4 seconds」并把 spinner 换成灰色对勾；展开后是分步清单（每步一行，活跃步转圈、完成步打勾，可带右侧次数/耗时/增删行数等次要信息）；时序用 setTimeout 序列驱动，跑完后仍可反复展开。四种变体：Steps（步骤清单）/Reasoning（一段会展开又收拢的推理文字）/Search（联网搜索轨迹：查询词+读过的来源）/Coding（工具轨迹：读过的文件、编辑、命令，带增删行数与等宽样式）。先看现有的折叠/状态列表组件，融入而不是覆盖。" },
  { file: "streaming-text.tsx", id: "streaming-text", title: "流式回答", titleEn: "Streaming Text",
    desc: "逐词流出的回答文本，带引用角标与后续追问建议。", descEn: "Word-by-word answer text with citation chips and follow-up suggestions.",
    category: "block", tags: ["ai-chat", "流式", "打字机", "beautifului", "引用"],
    demo: `import Comp from "./streaming-text";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「流式回答」装进我的项目：智能体逐词吐字的回答文本——每个词到达时带轻微模糊淡入（blur-in，不是硬蹦），行尾跟一个闪烁光标（caret-blink step-end，流光式），回答完成后光标消失；文本中间可插入引用角标（citation chip，hover 显示来源工具提示）；回答结束后淡入一排「追问建议」按钮（点击可填入输入框）；定格一段时间后重放。尊重 prefers-reduced-motion（词直接出现不模糊）。先看现有的消息渲染组件，融入而不是覆盖。" },
  { file: "approval-card.tsx", id: "approval-card", title: "审批卡", titleEn: "Approval Card",
    desc: "智能体请求执行敏感操作的审批卡：命令原文+三键决策+决策后状态。", descEn: "Approval card for sensitive agent actions with three-way decision.",
    category: "block", tags: ["ai-chat", "审批", "安全", "beautifului", "权限"],
    demo: `import Comp from "./approval-card";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「审批卡」装进我的项目：智能体请求执行敏感操作时的审批卡——琥珀色描边警示风格，含标题行（感叹标+「智能体请求执行命令」）、等宽字体命令原文块、一行影响范围说明，以及三颗决策按钮：仅本次允许（渐变实心）、永久允许、拒绝；决策后按钮区换成绿/红状态条（文案随决策变化）并带「撤销」链接可复位；容器 role=\"alertdialog\" 带无障碍标签，按钮保留 focus-visible。先看现有的确认弹层/权限组件，融入而不是覆盖。" },
  { file: "tool-chips.tsx", id: "tool-chips", title: "工具调用行", titleEn: "Tool Chips",
    desc: "多步工具调用轨迹：图标 chip + 状态，悬停出箭头、点开看细节。", descEn: "Multi-step tool trace rows with icons, states and expandable detail.",
    category: "block", tags: ["ai-chat", "工具调用", "轨迹", "beautifului", "折叠"],
    demo: `import Comp from "./tool-chips.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「工具调用行」装进我的项目：智能体工具调用轨迹组件——每行是一个 chip：左侧按工具类型分图标（思考=星芒、写文件=铅笔、跑命令=终端、读文件=文档，都用手写 SVG 路径不用图标库）、中间是工具名与一句话说明、右侧是状态（转圈→对勾）；悬停该行才浮现展开箭头；点开在下方展示这次调用到底做了什么（多行细节，可带 +N 增行绿色的等宽文本）。步骤按时序逐条推进（每步 ~700ms），跑完后仍可点开看历史。先看现有的轨迹/日志列表组件，融入而不是覆盖。" },
  { file: "task-rows.tsx", id: "task-rows", title: "任务清单行", titleEn: "Task Rows",
    desc: "任务行：环形进度扫过、展开细节、失败可重试、状态流转。", descEn: "Task rows with ring progress, expandable detail, retry on failure.",
    category: "block", tags: ["ai-chat", "任务", "进度", "beautifului", "重试"],
    demo: `import Comp from "./task-rows.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「任务清单行」装进我的项目：智能体多任务执行清单——每行左侧是一个环形进度指示（SVG circle stroke-dasharray 扫过，从 0 到 66%，完成后变对勾）；行内是任务名与状态文案；点行可在下方展开细节步骤（多行小字）；失败的行显示红色「Failed」与一颗重试按钮，点击后回到执行中转回完成；整体按时序演进（行 1 扫ring→展开→收起，行 2 失败→重试→完成），跑完后细节仍可点开。变体 Capsules（胶囊）/Rows（行式）。先看现有的任务列表组件，融入而不是覆盖。" },
  { file: "chat.tsx", id: "chat", title: "对话面板", titleEn: "Chat",
    desc: "带标签页、回复流与输入框的对话面板，用户发送后才有回复序列。", descEn: "Chat panel with tabs, reply sequence and composer.",
    category: "block", tags: ["ai-chat", "对话", "面板", "beautifului", "标签页"],
    demo: `import Comp from "./chat.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「对话面板」装进我的项目：一个可交互的对话面板——顶部标签页（可切换会话/模式），中部消息区：用户消息右对齐实底气泡，智能体回复逐条出现（每条带小标题+次要说明+耗时，出现时 fade-up 微位移）；回复到来前显示「正在处理」的模糊收缩态（opacity+blur+scale 三件套，cubic-bezier(0.23,1,0.32,1)）；底部是输入框与发送按钮，点发送才开始回复序列，序列跑完可再发。先看现有的聊天气泡/消息列表组件，融入而不是覆盖。" },
  { file: "prompt-bar.tsx", id: "prompt-bar", title: "提示输入条", titleEn: "Prompt Bar",
    desc: "带模型选择、附件、语音与 WebGL 扫光提交的智能体输入条。", descEn: "Agent composer with model picker, attachments, voice and a WebGL sweep.",
    category: "block", tags: ["ai-chat", "输入框", "composer", "beautifului", "webgl"],
    demo: `import Comp from "./prompt-bar.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「提示输入条」装进我的项目：智能体对话的输入条——圆角/胶囊两种外形；左侧 + 号展开附件菜单（可加截图/文件，加过的显示为可删的 chip）；右侧是模型选择器（点开是带说明的模型清单，当前项有对勾）与语音按钮；文本域随内容自动长高（测量 span 再设高度）；「自动」开关打开时输入条会自动演示打字并周期性提交，用户一碰就交还控制权；提交瞬间用 canvas + WebGL shader 在条上扫出一道高光（无 WebGL 时优雅降级为纯 CSS 高光）。先看现有的输入框/composer 组件，融入而不是覆盖。" },
  { file: "recommendation-card.tsx", id: "recommendation-card", title: "推荐卡", titleEn: "Recommendation Card",
    desc: "智能体给出多个选项的推荐卡：切换选中、可展开、可采纳。", descEn: "Recommendation card with selectable options and an accept action.",
    category: "block", tags: ["ai-chat", "推荐", "选择", "beautifului", "卡片"],
    demo: `import Comp from "./recommendation-card.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「推荐卡」装进我的项目：智能体主动给建议的推荐卡——卡头一句「要我帮你下单吗？」之类的问句；主体是若干候选选项（每项一行：标题+说明+右侧单选圆点），点击切换选中（选中项描边提亮）；「其他选项」可展开/收起余下候选；底部是「采纳」与「先不用」两颗按钮，采纳后卡片换成完成态（对勾+说明+撤销）。用现有卡片的圆角/描边/阴影语汇，键盘可达。先看现有卡片组件，融入而不是覆盖。" },
  { file: "context-cards.tsx", id: "context-cards", title: "上下文引用卡", titleEn: "Context Cards",
    desc: "智能体引用过的资料卡：来源徽标、正文片段、字符数与匹配度。", descEn: "Context cards citing retrieved sources with badges and match scores.",
    category: "block", tags: ["ai-chat", "引用", "资料", "beautifului", "检索"],
    demo: `import Comp from "./context-cards.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「上下文引用卡」装进我的项目：展示智能体检索到的资料——每张卡含紧挨标题上方的元信息行（匹配度百分比 + 操作按钮组，hover 才显现）、标题、两三行正文片段、底部一行带彩色圆点的来源（文件名 + 徽标 PDF/CSV，按类型配色）；卡片错时淡入（fade-up 依次进场）。用现有卡片的描边/圆角语汇。先看现有引用/来源展示组件，融入而不是覆盖。" },
  { file: "diff-table.tsx", id: "diff-table", title: "变更表", titleEn: "Diff Table",
    desc: "智能体提议的数据变更表：行删除动画、红色着色、逐行完成。", descEn: "Proposed data-change table with row removals and staged completion.",
    category: "block", tags: ["ai-chat", "diff", "表格", "beautifului", "变更"],
    demo: `import Comp from "./diff-table.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「变更表」装进我的项目：智能体提议的数据变更预览表——表头一行标题（如「Proposed menu cleanup」）与右侧操作；每行含名称、分类、站点与状态列；被删除的行先整体染红（背景+文字色调），随后以高度收起+淡出移出表体，底部统计「N 行将被移除」实时更新；剩下未变的行保持原样；三阶段时序（原始→染红→移出完成）可循环重放。先看现有的表格组件，融入而不是覆盖。" },
  { file: "records-table.tsx", id: "records-table", title: "数据表", titleEn: "Records Table",
    desc: "企业级数据表：首列吸附、列头排序菜单、行选中、底部统计。", descEn: "Data table with sticky first column, header sort menus, row selection and footer rollups.",
    category: "block", tags: ["ai-chat", "表格", "数据", "beautifului", "排序"],
    demo: `import Comp from "./records-table.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「数据表」装进我的项目：企业级数据表——表头可点开排序/筛选浮层（勾选式菜单，选中有对勾，浮层带轻微 pop-in），支持列排序指示；首列与表头吸附（position:sticky，横向滚动时不丢），行 hover 整行提亮、行可多选（自绘复选框，选中整行高亮）；单元格内容溢出省略；底部统计行（合计/平均，tabular 数字）与右侧「N 行」计数、滚动提示。表体自带细描边与圆角外壳，深色模式下一套完整配色。先看现有表格/数据网格组件，融入而不是覆盖。" },
  { file: "filter-table.tsx", id: "filter-table", title: "筛选表", titleEn: "Filter Table",
    desc: "任务表：顶部状态筛选 chips（带计数与颜色点），点选即时过滤。", descEn: "Task table with status filter chips that filter rows live.",
    category: "block", tags: ["ai-chat", "表格", "筛选", "beautifului", "任务"],
    demo: `import Comp from "./filter-table.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「筛选表」装进我的项目：任务列表+筛选——顶部一排筛选 chip（All / To do / In Progress / Completed，各带计数与状态色点，选中项实底反色），横向可滚动且隐藏滚动条；下方表格逐行：任务名、日期、状态（彩色点+文案）、负责人；点 chip 即时过滤行并保序；空结果时给一行温和提示。先看现有的列表/筛选组件，融入而不是覆盖。" },
  { file: "sidebar-nav.tsx", id: "sidebar-nav", title: "侧栏导航", titleEn: "Sidebar Nav",
    desc: "带滑动高亮块、搜索与分组的侧栏导航，右侧计数徽标。", descEn: "Sidebar nav with a sliding highlight, search and grouped items with badges.",
    category: "block", tags: ["ai-chat", "导航", "侧栏", "beautifului", "搜索"],
    demo: `import Comp from "./sidebar-nav.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「侧栏导航」装进我的项目：一个工作区侧栏——顶部搜索行（图标+输入框+快捷键提示）；下面是分组（Workspace / Objects）与条目列表，每项含图标、名称与右侧计数徽标（选中/悬停时徽标变实底）；高亮块是一个独立的圆角背景块，在条目间以测量后的位移/高度平滑滑动（transition），键盘可上下移动焦点；条目多了内部滚动。先看现有的侧栏/导航列表，融入而不是覆盖。" },
  { file: "search.tsx", id: "search", title: "搜索面板", titleEn: "Search",
    desc: "命令面板式搜索：输入即时过滤、键盘上下选择、回车确认、空态。", descEn: "Command-palette style search with live filtering and keyboard navigation.",
    category: "block", tags: ["ai-chat", "搜索", "命令面板", "beautifului", "键盘"],
    demo: `import Comp from "./search.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「搜索面板」装进我的项目：命令面板式搜索框——顶部一行（放大镜 + 输入框 + ESC 提示），下方结果列表；输入即时过滤（大小写不敏感），↑/↓ 移动选中项（选中项实底反色）回车确认；结果项含图标、标题与右侧次要说明；超过三个字符仍无结果时显示空态（「没有匹配项」+ 清除按钮）；列表最多显示若干条。角色用 combobox/listbox，键盘可达。先看现有搜索/命令面板组件，融入而不是覆盖。" },
  { file: "insight-cards.tsx", id: "insight-cards", title: "指标卡组", titleEn: "Insight Cards",
    desc: "指标卡：迷你折线图、数值跳动、可切换系列与时间窗。", descEn: "Metric cards with sparkline charts and switchable series.",
    category: "block", tags: ["ai-chat", "指标", "图表", "beautifului", "数据"],
    demo: `import Comp from "./insight-cards.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「指标卡组」装进我的项目：数据指标卡——每张卡含指标名、大号数值（tabular 数字，变化时数字滚动切换）、同比/环比涨跌（涨绿跌红带箭头），以及一条迷你折线图（带渐变面积、网格与 hover 十字光标+浮动数值气泡）；卡片可横向切换系列（左右箭头/圆点指示），也有时间窗切换；数值与曲线用同一份数据源派生，切换有过渡动画。先看现有的统计卡/图表组件，融入而不是覆盖。" },
  { file: "code-block.tsx", id: "code-block", title: "代码块", titleEn: "Code Block",
    desc: "智能体写码流式块：逐行流入、语法着色、复制按钮。", descEn: "Agent code block streaming line by line with syntax colors and copy.",
    category: "block", tags: ["ai-chat", "代码块", "流式", "beautifului", "复制"],
    demo: `import Comp from "./code-block.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「代码块」装进我的项目：智能体写代码时的流式代码块——代码逐行流出行出（每行 ~240ms，首行前稍等），关键字/字符串/函数名/注释四种着色，行号或左侧标记可选；顶部条含文件名与「复制」按钮（复制成功后短暂变「已复制」）；写完后停留数秒再重放。等宽字体，长行横向滚动不撑破卡片。先看现有代码块组件，融入而不是覆盖。" },
  { file: "fine-tune-card.tsx", id: "fine-tune-card", title: "参数微调卡", titleEn: "Fine-tune Card",
    desc: "让用户微调参数的卡：分段控件、数值输入、滑杆与重置。", descEn: "Parameter tuning card with segmented control, numeric fields and slider.",
    category: "block", tags: ["ai-chat", "参数", "微调", "beautifului", "表单"],
    demo: `import Comp from "./fine-tune-card.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「参数微调卡」装进我的项目：让用户直接微调智能体产物参数的卡——顶部标题与右侧菜单（点开是若干预设）；主体分区：分段控件（若干档位）、一对数值输入（宽/高，可输入可点步进）、一条滑杆（圆角/不透明度之类）+数值；任何改动后底部按钮从「保持默认」变为「应用」并可「重置」回默认值；整卡是现有卡片的描边圆角语汇。先看现有表单控件，融入而不是覆盖。" },
  { file: "selection-actions.tsx", id: "selection-actions", title: "划选操作条", titleEn: "Selection Actions",
    desc: "划选文字后的浮出操作条：改写/缩短/扩写，带打字与结果预览。", descEn: "Contextual bar under selected text with rewrite actions and typed previews.",
    category: "block", tags: ["ai-chat", "划选", "操作条", "beautifului", "编辑器"],
    extraFiles: ["atoms/Shimmer.tsx", "atoms/StreamText.tsx"],
    demo: `import Comp from "./selection-actions.tsx";\nexport default function Demo() { return <Comp />; }`,
    prompt: "请把「划选操作条」装进我的项目：文本划选后浮出的智能体操作条——选中的文字先被高亮（半透明主色背景，可换行断行时圆角依旧连续），下方浮出一条胶囊工具条（图标按钮 + 动作名：改写/缩短/扩写/翻译…）；点某个动作后条内出现输入式进度（先转圈「思考中」再流式打字），随后在下方给出改写结果卡（带「替换」「复制」按钮，替换后选区文本被替换掉，可撤销）；工具条用 requestAnimationFrame 批量测量，浮在选区最后一行的下方并与整体选区居中；Esc 或点击外部收起。先看现有的选区工具条/上下文菜单，融入而不是覆盖。" },
];

for (const entry of BEAUTIFULUI_COMPONENTS) {
  const files = [
    {
      name: entry.file.replace(/\.tsx$/, ".tsx"),
      content:
        licenseHeader({
          title: `${entry.title} · ${entry.file}`,
          file: `components/${entry.file}`,
          author: "Turbo（beautifului.dev）",
          copyright: "Copyright (c) 2026 Turbo",
          license: "MIT License",
          repo: "github.com/TurboKach/ai-native-react-components",
          sourceUrl: `https://github.com/TurboKach/ai-native-react-components/blob/main/components/${entry.file}`,
        }) + readSource(path.join(BEAUTIFULUI, "components", entry.file)),
    },
  ];
  for (const extra of entry.extraFiles ?? []) {
    files.push({
      name: path.basename(extra),
      content:
        licenseHeader({
          title: `${path.basename(extra)}（${entry.title} 的附属模块）`,
          file: `components/${extra}`,
          author: "Turbo（beautifului.dev）",
          copyright: "Copyright (c) 2026 Turbo",
          license: "MIT License",
          repo: "github.com/TurboKach/ai-native-react-components",
          sourceUrl: `https://github.com/TurboKach/ai-native-react-components/blob/main/components/${extra}`,
        }) + readSource(path.join(BEAUTIFULUI, "components", extra)),
    });
  }
  const id = `beautifului-${entry.id}`;
  const header = `/**
 * 「${entry.title}」逐字收录（V3-3 素材库）。
 *
 * 来源：Beautiful UI · github.com/TurboKach/ai-native-react-components · components/${entry.file}
 * 作者：Turbo（beautifului.dev）
 * 许可：MIT（Copyright (c) 2026 Turbo）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */
`;
  const body = `{
  id: ${ts(id)},
  title: ${ts(entry.title)},
  titleEn: ${ts(entry.titleEn)},
  description: ${ts(entry.desc)},
  descriptionEn: ${ts(entry.descEn)},
  category: ${ts(entry.category)},
  tags: ${ts(entry.tags)},
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: ${JSON.stringify(files.map((f) => ({ name: f.name, language: "tsx", content: f.content })), null, 2).replace(/\n/g, "\n  ")},
  prompt: ${ts(entry.prompt)},
  source: { site: "Beautiful UI", url: "https://github.com/TurboKach/ai-native-react-components", license: "MIT" },
  preview: {
    theme: "beautifului",
    demo: ${ts(entry.demo)},
  },
}`;
  writeCard(`${id}.ts`, id, header, body);
}

// ============================================================
// 2. RareUI（Codewithswappy/RareUI，MIT，全收）
// ============================================================
// 上游结构：components/rareui/<Name>/<Name>.tsx（部分件另有 index.ts 仅做再导出，不取）；
// 另有 5 件只在 public/r/*.json registry 里（ThreeDButton/FeatureBadge/AvatarGroup/
// GlassSearchBar/ProfileDropdown），解包到 <refs>/rareui-extracted/ 后一并收。

const RAREUI_COMPONENTS = [
  { dir: "AnimatedTab", file: "AnimatedTab.tsx", id: "animated-tab", title: "动效标签页", titleEn: "Animated Tabs",
    desc: "选中项滑动高亮块的标签页，悬停有预滑提示。", descEn: "Tabs with a sliding pill highlight and hover preview.",
    category: "control", tags: ["rareui", "标签页", "动效", "framer-motion", "导航"],
    demo: `import { useState } from "react";\nimport { AnimatedTabs } from "./AnimatedTab";\nexport default function Demo() {\n  const [tab, setTab] = useState("overview");\n  return <AnimatedTabs tabs={[{ id: "overview", label: "概述" }, { id: "api", label: "接口" }, { id: "settings", label: "设置" }]} activeTab={tab} onChange={setTab} />;\n}`,
    prompt: "请把「动效标签页」装进我的项目：一个受控的标签页组件（props: tabs[{id,label}] / activeTab / onChange）——选中项的圆角高亮块是独立元素，在标签间以 spring 平滑滑动到新位置（layoutId 共享布局）；鼠标悬停某一项时高亮块先预滑过去做提示，移开回到选中项；标签文字随选中状态变粗变色；容器是浅底描边的胶囊条，深色模式一套对应配色。先看现有的分段控件/标签页组件，融入而不是覆盖。" },
  { dir: "GlassShimmerButton", file: "GlassShimmerButton.tsx", id: "glass-shimmer-button", title: "玻璃流光按钮", titleEn: "Glass Shimmer Button",
    desc: "毛玻璃胶囊按钮，一道高光周期性斜切扫过。", descEn: "Glass pill button with a periodic diagonal shimmer sweep.",
    category: "control", tags: ["rareui", "按钮", "毛玻璃", "流光", "css"],
    demo: `import { GlassShimmerButton } from "./GlassShimmerButton";\nexport default function Demo() { return <GlassShimmerButton>Get Started</GlassShimmerButton>; }`,
    prompt: "请把「玻璃流光按钮」装进我的项目：一颗毛玻璃胶囊按钮——半透明深底 + backdrop-blur + 内描边，顶部一条 1px 白色渐变高光模拟玻璃棱；一道斜切（skewX）的白色高光带以 2.5s 周期从右向左扫过（宽度 200%、left 从 -50% 推），悬停时按钮轻微放大、阴影加重，按下缩回；深色模式反色（浅底深字）。要保留键盘 focus-visible 描边与 disabled 态。先看现有的按钮体系，融入而不是覆盖。" },
  { dir: "LiquidButton", file: "LiquidButton.tsx", id: "liquid-button", title: "液态按钮", titleEn: "Liquid Button",
    desc: "鼠标移动时液体跟随倾斜的按钮，可开启滴落动画。", descEn: "Button whose liquid surface tilts toward the cursor, with optional dripping.",
    category: "control", tags: ["rareui", "按钮", "液态", "motion", "交互"],
    demo: `import LiquidButton from "./LiquidButton";\nexport default function Demo() { return <LiquidButton text="Liquid" isDripping />; }`,
    prompt: "请把「液态按钮」装进我的项目：一颗液态质感的按钮——鼠标在按钮上移动时，内部液面/高光按鼠标相对位置（motion value + spring）倾斜与偏移，形成液体被拨动的效果；离开后弹簧回正；可选「滴落」模式：按下/悬停时从下沿滴下几滴同色液滴（独立动画后消失）。文字在液面之上保持清晰对比；容器圆角与阴影走现有语汇。先看现有的按钮体系，融入而不是覆盖。" },
  { dir: "LoadingSpinner", file: "LoadingSpinner.tsx", id: "loading-spinner", title: "脉冲加载环", titleEn: "Loading Spinner",
    desc: "同心弧线段旋转的加载环，多段错速。", descEn: "Concentric arc spinner with staggered rotating arcs.",
    category: "control", tags: ["rareui", "加载", "spinner", "framer-motion", "等待"],
    demo: `import LoadingSpinner from "./LoadingSpinner";\nexport default function Demo() { return <LoadingSpinner />; }`,
    prompt: "请把「脉冲加载环」装进我的项目：一个加载指示器——若干段同心圆弧各自以不同速度/方向旋转（framer-motion 驱动 strokeDasharray 或 rotate），叠出连续抽动的脉冲感；尺寸可通过 className 缩放；颜色取当前主色（currentColor）；在 prefers-reduced-motion 下降级为静态或极慢旋转。要能放在按钮里当 busy 指示（aria-busy + 视觉隐藏文案）。先看现有的加载组件，融入而不是覆盖。" },
  { dir: "LiquidTooltip", file: "LiquidTooltip.tsx", id: "liquid-tooltip", title: "液态提示气泡", titleEn: "Liquid Tooltip",
    desc: "带液体形变与视差跟手的提示气泡，四向可选。", descEn: "Springy liquid tooltip that tracks the cursor with parallax, four placements.",
    category: "control", tags: ["rareui", "tooltip", "弹簧", "framer-motion", "交互"],
    demo: `import { LiquidTooltip } from "./LiquidTooltip";\nexport default function Demo() {\n  return <LiquidTooltip text="保存并运行"><button type="button" className="rounded-lg border border-neutral-300 px-3 py-1.5 text-sm dark:border-neutral-700 dark:text-white">悬停我</button></LiquidTooltip>;\n}`,
    prompt: "请把「液态提示气泡」装进我的项目：一个 tooltip（props: text / placement: top|bottom|left|right）——悬停或聚焦时气泡以弹簧（stiffness 260 / damping 20）从锚点方向弹出并带轻微过冲；鼠标在触发元素上移动时，气泡内部元素有视差跟手位移（motion value + useTransform），离开时收回带阻尼；气泡是实底反色圆角卡带小箭头，role=\"tooltip\" 与 aria-describedby 接上。先看现有的 tooltip 体系，融入而不是覆盖。" },
  { dir: "PremiumButton", file: "PremiumButton.tsx", id: "premium-button", title: "高级渐变按钮", titleEn: "Premium Button",
    desc: "柠檬绿多段渐变按钮，悬停时渐变流动+箭头内推。", descEn: "Lime multi-stop gradient button whose gradient slides on hover.",
    category: "control", tags: ["rareui", "按钮", "渐变", "css", "悬停"],
    demo: `import PremiumButton from "./PremiumButton";\nexport default function Demo() { return <PremiumButton />; }`,
    prompt: "请把「高级渐变按钮」装进我的项目：一颗柠檬绿渐变按钮——背景是多段重复的 linear-gradient（300% 尺寸）停在左侧，悬停时 background-position 移到右侧，形成「渐变往里流」的观感；文字带同色系 text-shadow 做发光；左侧是一枚自绘 SVG 图标，悬停时图标位移/放大；整颗按钮阴影是同色低透明度扩散。先看现有的主按钮体系，融入而不是覆盖。" },
  { dir: "SoftButton", file: "SoftButton.tsx", id: "soft-button", title: "新拟态软按钮", titleEn: "Soft Button",
    desc: "新拟态（neumorphism）按钮：外凸阴影，悬停变实底并拉字距。", descEn: "Neumorphic button with layered shadows that inverts on hover.",
    category: "control", tags: ["rareui", "按钮", "新拟态", "阴影", "悬停"],
    demo: `import SoftButton from "./SoftButton";\nexport default function Demo() { return <SoftButton>Press me</SoftButton>; }`,
    prompt: "请把「新拟态软按钮」装进我的项目：一颗 neumorphism 按钮——多层阴影叠出外凸立体（右下白高光 + 左上灰影 + 两道内阴影），整体随 600ms 缓动；悬停时背景变实黑、文字变白、字距从 1px 拉到 2px 并放大 5%，阴影换成同色发光；按下缩到 0.98。深色模式一套对应阴影（白高光降到 0.05，黑影加重）。先看现有按钮体系，融入而不是覆盖。" },
  { dir: "Neumorphism3DButton", file: "Neumorphism3DButton.tsx", id: "neumorphism-3d-button", title: "新拟态 3D 按钮", titleEn: "Neumorphism 3D Button",
    desc: "按下时凹陷回弹的新拟态圆钮，尺寸随字号。", descEn: "Neumorphic round button that presses in and springs back.",
    category: "control", tags: ["rareui", "按钮", "新拟态", "3d", "按下"],
    demo: `import { Neumorphism3DButton } from "./Neumorphism3DButton";\nexport default function Demo() { return <Neumorphism3DButton>Click Me</Neumorphism3DButton>; }`,
    prompt: "请把「新拟态 3D 按钮」装进我的项目：一颗圆形 neumorphism 按钮——静息时靠 -0.15em/-0.15em 白高光与右下黑影浮起；按下（mousedown/touch）时阴影反向/收小，视觉上一按就凹进去，松开弹回；尺寸全部用 em 因而跟随字体大小（放进不同容器自动适配）；深色模式一套对应阴影。带 disabled 态与 focus-visible 描边，键盘 Enter/Space 同样触发按下反馈。先看现有按钮体系，融入而不是覆盖。" },
  { dir: "RetroPixelButton", file: "RetroPixelButton.tsx", id: "retro-pixel-button", title: "复古像素按钮", titleEn: "Retro Pixel Button",
    desc: "悬停时像素方块从底沿滚过的等宽字体按钮。", descEn: "Mono retro button with a pixel block rolling across on hover.",
    category: "control", tags: ["rareui", "按钮", "像素", "复古", "等宽"],
    demo: `import RetroPixelButton from "./RetroPixelButton";\nexport default function Demo() { return <RetroPixelButton />; }`,
    prompt: "请把「复古像素按钮」装进我的项目：一颗复古像素风按钮——等宽字体全大写文字，浅底细描边圆角矩形；悬停时一小块方形像素（可配颜色）从左侧沿底沿滚到右侧，同时文字轻微位移（像被像素块推着走）；按下缩到 0.98。像素块颜色/底色/文字色都可通过 props 覆盖。先看现有按钮体系，融入而不是覆盖。" },
  { dir: "FeatureBadge", file: "FeatureBadge.tsx", fromRegistry: true, id: "feature-badge", title: "发光徽标", titleEn: "Feature Badge",
    desc: "胶囊徽标：白色小签+说明文字，一道高光周期扫过。", descEn: "Pill badge with a white tag and a periodic glare sweep.",
    category: "control", tags: ["rareui", "徽标", "高光", "framer-motion", "标签"],
    demo: `import FeatureBadge from "./FeatureBadge";\nexport default function Demo() { return <FeatureBadge badgeText="New">Multi-currency account</FeatureBadge>; }`,
    prompt: "请把「发光徽标」装进我的项目：一个「新功能」提示徽标——毛玻璃胶囊（半透明底 + backdrop-blur + 细描边），左侧白色小圆角签写短标签（如 New），右侧说明文字；一道 50% 宽的透明-白-透明渐变高光带以 skewX 斜切，从左侧周期扫到右侧（带 repeatDelay 停顿），悬停时底色与描边略提亮、文字变白；整件可作为链接。先看现有的徽标/标签组件，融入而不是覆盖。" },
  { dir: "FloatingNavigation", file: "FloatingNavigation.tsx", id: "floating-navigation", title: "悬浮导航条", titleEn: "Floating Navigation",
    desc: "底部悬浮导航：滑动指示块、图标+标签、方向键可达。", descEn: "Floating bottom nav with a sliding indicator and arrow-key support.",
    category: "control", tags: ["rareui", "导航", "悬浮", "framer-motion", "键盘"],
    demo: `import FloatingNavigation from "./FloatingNavigation";\nexport default function Demo() { return <FloatingNavigation />; }`,
    prompt: "请把「悬浮导航条」装进我的项目：贴在底部的悬浮导航——毛玻璃胶囊容器（backdrop-blur-xl + 半透明底 + 大阴影），内含若干条目（自绘 SVG 图标 + 标签）；选中项的圆角指示块以 spring 在条目间滑动（含宽度自适应）；悬停时该条目标签上浮淡入；←/→ 方向键在条目间移动并同步焦点（role=\"tablist\" + roving tabindex）；深色模式反色。先看现有的底部/侧边导航，融入而不是覆盖。" },
  { dir: "ToastTabs", file: "ToastTabs.tsx", id: "toast-tabs", title: "环形进度标签卡", titleEn: "Toast Tabs",
    desc: "头像+引言卡：环形进度倒计时自动轮播，悬停暂停。", descEn: "Avatar quote cards with a circular countdown ring and autoplay.",
    category: "block", tags: ["rareui", "轮播", "环形进度", "framer-motion", "评价"],
    demo: `import ToastTabs from "./ToastTabs";\nexport default function Demo() { return <ToastTabs />; }`,
    patches: offlineImagePatches(
      [
        "https://images.unsplash.com/photo-1599566150163-29194dcaad36?q=80&w=200&auto=format&fit=crop",
        "https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?q=80&w=200&auto=format&fit=crop",
        "https://images.unsplash.com/photo-1633332755192-727a05c4013d?q=80&w=200&auto=format&fit=crop",
      ],
      { avatar: true },
    ),
    prompt: "请把「环形进度标签卡」装进我的项目：一组可轮播的短评卡——每张含圆头像、姓名、引言，以及头像外圈一道环形进度（SVG stroke-dashoffset 随倒计时推进，用 motion value 驱动不触发 React 重渲染）；到点自动切下一张（卡片淡出上浮、新卡从下方进场）；鼠标悬停在卡上暂停倒计时，离开续走；卡片下沿一排指示点，点击直接跳转。先看现有的轮播/卡片组件，融入而不是覆盖。" },
  { dir: "ParticleCard", file: "ParticleCard.tsx", id: "particle-card", title: "粒子显影名片", titleEn: "Particle Card",
    desc: "悬停时粒子聚成头像的名片，移开散回网格。", descEn: "Profile card whose particles converge into the avatar on hover.",
    category: "block", tags: ["rareui", "粒子", "名片", "motion", "交互"],
    demo: `import ParticleCard from "./ParticleCard";\nexport default function Demo() { return <ParticleCard name="Sam Jenkins" role="Product Designer" bio="专注交互与设计系统的前端工程师。" tags={["UI/UX", "React", "Motion"]} />; }`,
    patches: offlineImagePatches(
      ["https://images.unsplash.com/photo-1676377630534-a08fd9778701?q=80&w=930&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D"],
      { avatar: true },
    ),
    dark: true,
    prompt: "请把「粒子显影名片」装进我的项目：一张有粒子图层的人像名片——卡片按下/悬停时，按网格（cols×rows）铺开的细密粒子从原位飞向头像区域，拼出照片轮廓（粒子位置用 motion value 逐帧插值，悬停再散回网格）；卡片本身是深色磨砂圆角卡：上面是头像、姓名、职位、一段简介与几枚技能标签，出场时元素依次 fade-up；离开时粒子与内容一起回落。粒子数可配（默认 20×24），移动端降级为静态图。先看现有的个人卡/头像组件，融入而不是覆盖。" },
  { dir: "PremiumProfileCard", file: "PremiumProfileCard.tsx", id: "premium-profile-card", title: "3D 翻转名片", titleEn: "Premium Profile Card",
    desc: "跟随鼠标倾斜、可翻转看背面的玻璃名片。", descEn: "Glass profile card that tilts toward the cursor and flips to a back face.",
    category: "block", tags: ["rareui", "名片", "3D", "翻转", "悬停"],
    demo: `import PremiumProfileCard from "./PremiumProfileCard";\nexport default function Demo() { return <PremiumProfileCard />; }`,
    // 上游用两张远程图（噪点纹理 + 头像）当装饰，预览里换内联占位图
    patches: [
      { from: "https://grainy-gradients.vercel.app/noise.svg", to: NOISE_URI, note: "预览离线化：噪点纹理换内联 SVG" },
      {
        from: "https://images.unsplash.com/photo-1506794778202-cad84cf45f1d?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
        to: AVATAR_URI,
        note: "预览离线化：头像换内联 SVG",
      },
    ],
    prompt: "请把「3D 翻转名片」装进我的项目：一张会跟随鼠标倾斜的玻璃名片——鼠标在卡上移动时，整卡以 max 10° 的幅度绕 X/Y 轴轻微转动（逐帧插值收敛到目标角度，带摩擦阻尼），移到卡外回正；卡面是磨砂玻璃质感（半透明底 + 内高光 + 细描边 + 噪点叠加），含头像、姓名、职位与一排联系方式图标；点右下角按钮整卡绕 Y 轴翻到背面（3D 翻转 + 正面隐藏），背面是技能/工具/语言的进度条清单，可翻回。移动端无悬停时保持静态、仅保留翻转。先看现有的名片/卡片组件，融入而不是覆盖。" },
  { dir: "SoundText", file: "SoundText.tsx", id: "sound-text", title: "发声文字", titleEn: "Sound Text",
    desc: "逐字悬停发声的文字：音高按元音映射，配波形反馈。", descEn: "Text whose letters play a note on hover, with a waveform readout.",
    category: "text-animation", tags: ["rareui", "文字", "音效", "五声音阶", "悬停"],
    demo: `import SoundText from "./SoundText";\nexport default function Demo() { return <SoundText text="Hover over me" />; }`,
    prompt: "请把「发声文字」装进我的项目：一段逐字发声的文字——鼠标划过每个字母时该字母弹跳（framer-motion 控制 y/scale 回弹）并用 Web Audio API 的 OscillatorNode 发一个短音（音高按五声音阶比例映射到元音，基准频率可配 basePitch，默认 300Hz）；每个音的 ADSR 用 GainNode 塑形避免爆音，多次悬停复用同一个 AudioContext（首次用户手势时才创建/恢复，符合自动播放策略）；文字下方或一侧可给一条随声音抖动的波形条。无音频权限时静默降级，只保留视觉弹跳。先看现有文字动效组件，融入而不是覆盖。" },
  { dir: "WordMagnet", file: "WordMagnet.tsx", id: "word-magnet", title: "磁力文字", titleEn: "Word Magnet",
    desc: "单词被鼠标推开又弹回原位的磁力场文字。", descEn: "Words repelled by the cursor and sprung back into place.",
    category: "text-animation", tags: ["rareui", "文字", "磁力", "弹簧", "交互"],
    demo: `import WordMagnet from "./WordMagnet";\nexport default function Demo() { return <WordMagnet text="Move your cursor through these words" />; }`,
    prompt: "请把「磁力文字」装进我的项目：一段磁力场文字——鼠标靠近某词时，该词按距离反比的力度被推开（半径/力度/阻尼/回位延迟都可配：radius 默认 130、force 0.45、damping 28、returnDelay 400ms），鼠标离开后以 spring 弹回原位；位移用 motion value 驱动避免重渲染，鼠标移动节流到 ~60fps；单词按空格切分、逐个 span，其余排版参数（字号/字重/字距/行高/颜色）可整体传入。先看现有的文字动效组件，融入而不是覆盖。" },
  { dir: "MagneticScatterText", file: "MagneticScatterText.tsx", id: "magnetic-scatter-text", title: "磁性散字", titleEn: "Magnetic Scatter Text",
    desc: "悬停时字母被磁力吸引聚拢的标题文字。", descEn: "Headline letters that gather toward the cursor.",
    category: "text-animation", tags: ["rareui", "文字", "磁力", "framer-motion", "标题"],
    demo: `import { MagneticScatterText } from "./MagneticScatterText";\nexport default function Demo() { return <MagneticScatterText text="MAGNETIC" />; }`,
    prompt: "请把「磁性散字」装进我的项目：一段标题文字——默认状态每个字母各自小幅散开（随机偏移与旋转），鼠标进入文字区域时所有字母以磁力吸引的观感向鼠标位置聚拢并轻微放大，移开后弹回散开态；用 framer-motion 的动画序列驱动（visible/hidden 两套 variant，错峰 stagger）；键盘聚焦（tabindex + focus）同样触发聚拢，保证非鼠标可用。先看现有的标题/文字动效组件，融入而不是覆盖。" },
  { dir: "VaporSmokeText", file: "VaporSmokeText.tsx", id: "vapor-smoke-text", title: "烟雾散字", titleEn: "Vapor Smoke Text",
    desc: "逐字从模糊烟雾中凝聚成字的标题。", descEn: "Title letters that condense out of blurry smoke.",
    category: "text-animation", tags: ["rareui", "文字", "烟雾", "入场", "标题"],
    demo: `import { VaporSmokeText } from "./VaporSmokeText";\nexport default function Demo() { return <VaporSmokeText text="Smoke" />; }`,
    prompt: "请把「烟雾散字」装进我的项目：一段标题入场动效——每个字母初始是放大且高度模糊的「烟雾」态（filter: blur + 透明度 0 + 轻微上移/旋转），随后逐个（staggerChildren 0.08）凝聚到清晰原位，整段依次成形；字形上可加一点跟踪（letter-spacing）微调收束感；trigger 关闭时整体回到烟雾态便于重放。尊重 prefers-reduced-motion（直接显示）。先看现有的文字入场组件，融入而不是覆盖。" },
  { dir: "ImageExpandTestimonial", file: "ImageExpandTestimonial.tsx", fromRegistry: true, id: "image-expand-testimonial", title: "展开式评价轮播", titleEn: "Image Expand Testimonial",
    desc: "一排条带图：悬停展开、自动轮播、星评与箭头。", descEn: "Expanding image strip testimonials with autoplay and star ratings.",
    category: "block", tags: ["rareui", "轮播", "评价", "图片", "悬停"],
    demo: `import { ImageExpandTestimonial } from "./ImageExpandTestimonial";\nconst ITEMS = [\n  { id: "1", name: "Ava Chen", role: "Design Lead", company: "Northwind", quote: "组件质量远超预期，接入只用了一个下午。", image: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 300'%3E%3Crect width='200' height='300' fill='%23c7d2fe'/%3E%3Ccircle cx='100' cy='110' r='42' fill='%236366f1'/%3E%3Crect x='40' y='180' width='120' height='14' rx='7' fill='%236366f1'/%3E%3C/svg%3E", rating: 5 },\n  { id: "2", name: "Ben Ortiz", role: "Founder", company: "Cobalt", quote: "专注细节，动效顺滑得让人上瘾。", image: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 300'%3E%3Crect width='200' height='300' fill='%23bbf7d0'/%3E%3Ccircle cx='100' cy='110' r='42' fill='%2316a34a'/%3E%3Crect x='40' y='180' width='120' height='14' rx='7' fill='%2316a34a'/%3E%3C/svg%3E", rating: 4 },\n  { id: "3", name: "Chloe Park", role: "PM", company: "Lumen", quote: "真的省下了好几周的打磨时间。", image: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 200 300'%3E%3Crect width='200' height='300' fill='%23fed7aa'/%3E%3Ccircle cx='100' cy='110' r='42' fill='%23ea580c'/%3E%3Crect x='40' y='180' width='120' height='14' rx='7' fill='%23ea580c'/%3E%3C/svg%3E", rating: 5 },\n];\nexport default function Demo() { return <ImageExpandTestimonial testimonials={ITEMS} />; }`,
    prompt: "请把「展开式评价轮播」装进我的项目：一排竖向条带图（props: testimonials[{id,name,role,company,quote,image,rating}]）——悬停某条时该条横向展开露出完整图与评价文字，其余条收缩变暗；不悬停时按 autoPlayInterval（默认 5s）自动轮播下一条；条带上叠人物名/公司/星级（自绘 SVG 星）；两侧给上一张/下一张箭头按钮（键盘可达），鼠标进入容器暂停自动播放。先看现有的轮播/评价组件，融入而不是覆盖。" },
  { dir: "Book3D", file: "Book3D.tsx", id: "book-3d", title: "3D 翻书", titleEn: "Book 3D",
    desc: "封面翻开、书页依次翻动的 3D 书本组件。", descEn: "3D book with a cover that opens and pages that turn.",
    category: "block", tags: ["rareui", "3d", "书", "翻页", "framer-motion"],
    demo: `import { Book3D } from "./Book3D";\nexport default function Demo() { return <Book3D title="RareUI" subtitle="Component Book" />; }`,
    // 上游默认书页里有一张远程图 + 两处纸张纹理贴图，预览里换内联占位
    patches: [
      {
        from: "https://images.unsplash.com/photo-1589561253898-768105ca91a8?q=80&w=1738&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
        to: PHOTO_URI,
        note: "预览离线化：书页图片换内联 SVG",
      },
      { from: "https://www.transparenttextures.com/patterns/paper.png", to: NOISE_URI, note: "预览离线化：纸张纹理换内联 SVG" },
    ],
    prompt: "请把「3D 翻书」装进我的项目：一本可交互的 3D 书——合上时只见封面（书脊在左，轻微透视与阴影）；点击/拖拽封面右沿后封面绕书脊轴翻开，露出第一页；随后可逐页翻动（每页绕左侧书脊 rotateY，翻到背面时 z-index 与正反面切换，翻页带惯性轻微回弹）；书页内容支持标题/正文/图片/引言四种块（props pages[]），页码与阴影随翻页实时变化；整书尺寸可配（width/height）。prefers-reduced-motion 时改为直接切换页码。先看现有的卡片/翻页组件，融入而不是覆盖。" },
  { dir: "LiquidMetal", file: "LiquidMetal.tsx", id: "liquid-metal", title: "液态金属字", titleEn: "Liquid Metal",
    desc: "WebGL 液化的金属质感文字/图形，可配参数。", descEn: "WebGL liquid-metal text with tweakable refraction and edge params.",
    category: "background", tags: ["rareui", "webgl", "液态金属", "着色器", "文字"],
    demo: `import LiquidMetal from "./LiquidMetal";\nexport default function Demo() { return <LiquidMetal text="RareUI" />; }`,
    prompt: "请把「液态金属字」装进我的项目：一段 WebGL 液态金属标题——用 canvas + WebGL2 渲染：把文字/图形先画进离屏 canvas 取纹理，再用片元着色器在纹理上做位移与折射扰动（dispersion 控制色散、edge 控制边缘锐化、patternBlur 控制图案模糊、liquify 控制液化强度、patternScale 控制图案密度、speed 控制时间流速），金属感来自多频噪声叠加+边缘高光；鼠标位置可微扰（可选）。无 WebGL2 时优雅降级为静态文字。可配 imageSource 换成任意图形。先看现有的 canvas/webgl 背景组件，融入而不是覆盖。" },
  { dir: "LiquidWave", file: "LiquidWave.tsx", id: "liquid-wave", title: "交互液面（three.js）", titleEn: "Liquid Wave",
    desc: "three.js 写的可拨动液面：鼠标搅动、自演示、粘性/泊松可调。", descEn: "Three.js interactive liquid surface with mouse stirring and auto demo.",
    category: "background", tags: ["rareui", "threejs", "webgl", "液面", "交互"],
    demo: `import LiquidWave from "./LiquidWave";\nexport default function Demo() { return <LiquidWave color1="#38bdf8" color2="#818cf8" color3="#f472b6" />; }`,
    prompt: "请把「交互液面」装进我的项目：一块可拨动的液面背景（基于 three.js）——全屏 quad 上用 shader 求解流体方程（可选 viscous/BFECC 更稳）、鼠标移动时在光标处注入力（mouseForce 力度、cursorSize 笔刷半径），液面泛起彩色涟漪并缓慢耗散；提供 autoDemo 自演示（无操作一段时间后自动游走搅动，autoSpeed/autoIntensity/autoRampDuration 可调，用户一动立刻接管）；用 IntersectionObserver 在不可见时暂停渲染、ResizeObserver 跟随容器；无 WebGL 时退化为渐变底。先看现有的背景动效组件，融入而不是覆盖。" },
  { dir: "ThreeDButton", file: "ThreeDButton.tsx", fromRegistry: true, id: "three-d-button", title: "3D 立体按钮", titleEn: "3D Button",
    desc: "多层内阴影叠出真实厚度的深色立体按钮。", descEn: "Dark 3D button built from stacked inset shadows.",
    category: "control", tags: ["rareui", "按钮", "3d", "阴影", "framer-motion"],
    demo: `import ThreeDButton from "./ThreeDButton";\nexport default function Demo() { return <ThreeDButton text="Book a call" />; }`,
    prompt: "请把「3D 立体按钮」装进我的项目：一颗深色立体按钮——厚度感来自多层叠加阴影（内顶 1px 白高光 + 一圈 1px 描边 + 由近及远 6 层愈拉愈长的暗影，模拟悬浮在深底上的实心块）；悬停时按钮轻微上浮（y 位移）阴影随之拉长，按下时压回并缩到 0.98（spring 400/15）；文字保持高对比。link 形态（传 href 时渲染为链接）与按钮形态都要，键盘 focus-visible 有描边。先看现有按钮体系，融入而不是覆盖。" },
  { dir: "ProfileDropdown", file: "ProfileDropdown.tsx", fromRegistry: true, id: "profile-dropdown", title: "账户下拉", titleEn: "Profile Dropdown",
    desc: "带账户切换与菜单项高亮滑块的账户下拉面板。", descEn: "Account dropdown with an account switcher and sliding row highlight.",
    category: "block", tags: ["rareui", "下拉", "账户", "framer-motion", "菜单"],
    demo: `import ProfileDropdown from "./ProfileDropdown";\nexport default function Demo() { return <ProfileDropdown />; }`,
    patches: offlineImagePatches(
      [
        "https://api.dicebear.com/9.x/notionists/svg?seed=Sophia",
        "https://api.dicebear.com/9.x/open-peeps/svg?seed=Ryan",
        "https://api.dicebear.com/9.x/notionists/svg?seed=Riley",
      ],
      { avatar: true },
    ),
    prompt: "请把「账户下拉」装进我的项目：一个账户下拉——触发区是带头像、名称与套餐的按钮（右侧下拉箭头，展开时旋转）；点开后弹出面板：顶部分组标题，中间菜单项（图标+文案，选中项有实底高亮块以 layoutId 平滑滑动到新位置）；下方是「切换账户」区，可展开账户列表（头像+名称+套餐，当前项打勾），点选后触发区随之更新；面板带 pop-in 弹出与点外部关闭（Esc 也可），带 aria-expanded/aria-haspopup。先看现有下拉菜单组件，融入而不是覆盖。" },
  { dir: "GlassSearchBar", file: "GlassSearchBar.tsx", fromRegistry: true, id: "glass-search-bar", title: "玻璃搜索栏", titleEn: "Glass Search Bar",
    desc: "毛玻璃搜索：下拉即时结果、键盘选择、空态。", descEn: "Frosted-glass search with live results, keyboard nav and an empty state.",
    category: "control", tags: ["rareui", "搜索", "毛玻璃", "键盘", "framer-motion"],
    demo: `import GlassSearchBar from "./GlassSearchBar";\nexport default function Demo() { return <GlassSearchBar />; }`,
    prompt: "请把「玻璃搜索栏」装进我的项目：一个毛玻璃搜索栏——外层半透明模糊包裹、内层实底输入核心（聚焦时外圈 ring 高亮）；输入即时过滤出下拉结果（每项图标+名称+小箭头），↑/↓ 选择、回车打开（调用注入的 onSelect，不硬编码路由）、Esc 关闭；无结果时给空态文案；结果过多时内部滚动，下拉带轻微高度/透明度过渡。role=\"combobox\" 语义完整、键盘可达。先看现有搜索组件，融入而不是覆盖。" },
  { dir: "AvatarGroup", file: "AvatarGroup.tsx", fromRegistry: true, id: "avatar-group", title: "头像堆叠", titleEn: "Avatar Group",
    desc: "一排头像依次跳起并放大，选中者高亮前移。", descEn: "Row of avatars that hop and scale up in turn.",
    category: "block", tags: ["rareui", "头像", "动效", "framer-motion", "团队"],
    demo: `import AvatarGroup from "./AvatarGroup";\nexport default function Demo() { return <AvatarGroup />; }`,
    patches: offlineImagePatches(
      [
        "https://api.dicebear.com/9.x/notionists/svg?seed=Robert",
        "https://api.dicebear.com/9.x/notionists/svg?seed=Sophia",
        "https://api.dicebear.com/9.x/notionists/svg?seed=Liliana",
        "https://api.dicebear.com/9.x/notionists/svg?seed=Brian",
      ],
      { avatar: true },
    ),
    prompt: "请把「头像堆叠」装进我的项目：一排头像的动效展示——若干圆头像横向相邻（重叠一点），每隔 2s 轮换一位：被选中的头像上跳并放大（spring 回弹，底部对齐因此有跳跃空间）、外圈加一圈高亮描边，未选中的缩回原尺寸；头像用对象数组配置（src/alt，可换成我们自己的图片或首字母兜底）；视觉上是无框的干净横排，可加在团队列表/在线成员处。先看现有的头像组件，融入而不是覆盖。" },
];
for (const entry of RAREUI_COMPONENTS) {
  const rawFiles = entry.fromRegistry
    ? readRegistry(entry.dir)
    : [readSource(path.join(RAREUI, "components/rareui", entry.dir, entry.file))].map((content) => ({
        name: entry.file,
        content,
      }));
  // 图纸逐字，但每份都带上游版权与许可声明（MIT 要求随附）
  const files = rawFiles.map((f) => ({
    name: f.name,
    language: "tsx",
    content:
      licenseHeader({
        title: `${entry.title} · ${f.name}`,
        file: `components/rareui/${entry.dir}/${f.name}`,
        author: "Swapnil Kalambe（RareUI）",
        copyright: "Copyright (c) 2025 Swapnil Kalambe (RareUI)",
        license: "MIT License",
        repo: "github.com/Codewithswappy/RareUI",
        sourceUrl: `https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/${entry.dir}/${f.name}`,
      }) + f.content,
  }));
  const id = `rareui-${entry.id}`;
  const header = `/**
 * 「${entry.title}」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/${entry.dir}/${entry.file}
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */
`;
  const body = `{
  id: ${ts(id)},
  title: ${ts(entry.title)},
  titleEn: ${ts(entry.titleEn)},
  description: ${ts(entry.desc)},
  category: ${ts(entry.category)},
  tags: ${ts(entry.tags)},
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: ${JSON.stringify(files, null, 2).replace(/\n/g, "\n  ")},
  prompt: ${ts(entry.prompt)},
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: ${ts(entry.demo)},
  },
}`;
  writeCard(`${id}.ts`, id, header, body);
}

// ============================================================
// 3. UIverse（uiverse-io/galaxy，CC BY 4.0，精选 14 件）
// ============================================================
// 上游：每个文件是一段可独立运行的 HTML 片段（标记 + <style>），文件头注释带作者。
// 许可证：仓库 LICENSE 是 MIT（Copyright (c) 2023 Uiverse.io），站点条款为 CC BY 4.0、
// 要求给作者与 Uiverse.io 署名——source.license 同时写明，作者署名进卡头注释。

const UIVERSE_COMPONENTS = [
  { file: "Buttons/0x-Sarthak_hungry-penguin-30.html", id: "cta-arrow-button", title: "箭头滑出按钮", titleEn: "Arrow Slide CTA",
    desc: "悬停时白色圆点铺满整颗按钮、箭头滑出的 CTA。", descEn: "CTA button where a white dot floods the fill and the arrow slides out.",
    tags: ["uiverse", "按钮", "悬停", "cta", "css"],
    prompt: "请把「箭头滑出按钮」装进我的项目：一颗 CTA 按钮——紫色实底胶囊 + 3px 同色描边，文字全大写带字距，右侧一枚自绘箭头 SVG；静息时按钮右端叠着一个白色圆角方块（像光点），悬停时该白块横向铺满整颗按钮（0.8s cubic-bezier 缓动）并把底色染成深色，箭头同步向左滑入归位（位移 5px→0）；按下缩到 0.95。白块用 ::before 实现，按钮保持真 button 语义与 focus-visible。先看现有主按钮体系，融入而不是覆盖。" },
  { file: "Buttons/AlimurtuzaCodes_average-liger-0.html", id: "sparkle-generate-button", title: "星芒生成按钮", titleEn: "Sparkle Generate Button",
    desc: "悬停整颗按钮燃起紫光谱光、星芒放大的生成按钮。", descEn: "Generate button that ignites with a purple glow and enlarges its sparkle.",
    category: "control", tags: ["uiverse", "按钮", "渐变", "发光", "ai"],
    prompt: "请把「星芒生成按钮」装进我的项目：一颗「生成」按钮——深灰实底大圆角胶囊，左侧一枚自绘星芒 SVG（大小星四角形）+ 粗体文字；悬停时底色切成紫罗兰渐变（#A47CF3→#683FEA），星芒放大 1.2 并变纯白，文字变白，同时叠加四层阴影：内顶白色高光、内底暗色、外圈 4px 半透明白环、以及 180px 的大范围紫色泛光；整卡上浮 2px。按下回到平面。先看现有主按钮体系，融入而不是覆盖。" },
  { file: "Buttons/abuayaan01_tall-mayfly-66.html", id: "liquid-fill-button", title: "液面填充按钮", titleEn: "Liquid Fill Button",
    desc: "悬停时蓝色液面从底部涨满按钮的水波按钮。", descEn: "Button whose blue liquid rises to fill it on hover.",
    tags: ["uiverse", "按钮", "水波", "填充", "css"],
    prompt: "请把「液面填充按钮」装进我的项目：一颗水波填充按钮——黑色实底胶囊，内部藏一个蓝色方块作为「液面」；静息时液面停在按钮下方（不可见）；悬停时液面上升铺满整颗按钮（约 0.5s 缓动）并让文字变成深色；移开后液面退回；文字始终在上层保持可读。液面用独立元素 + overflow:hidden 实现，不靠背景切换。先看现有按钮体系，融入而不是覆盖。" },
  { file: "Buttons/ahmedgamal-hub_yellow-cheetah-82.html", id: "rainbow-glow-button", title: "彩虹光晕按钮", titleEn: "Rainbow Glow Button",
    desc: "悬停时彩虹光带绕按钮流动的发光按钮。", descEn: "Button with a rainbow gradient halo that flows on hover.",
    tags: ["uiverse", "按钮", "彩虹", "发光", "css"],
    prompt: "请把「彩虹光晕按钮」装进我的项目：一颗发光按钮——深黑实底圆角矩形，背后藏一条 400% 宽的彩虹线性渐变（红橙黄绿青蓝紫循环）作为光晕层（::before，blur(5px)，向四周各扩 2px）；静息时光晕透明度 0，悬停时淡入并让渐变背景位沿 x 轴 20s 线性循环流动，形成绕按钮奔流的光带；按下时光晕收起、文字转深色。文字保持纯白粗体。先看现有按钮体系，融入而不是覆盖。" },
  { file: "loaders/A-nshuman_fluffy-fox-90.html", id: "block-mosaic-loader", title: "方块呼吸加载器", titleEn: "Block Mosaic Loader",
    desc: "四个方块轮流伸缩的网格加载器。", descEn: "Four-block grid loader where blocks flex in turn.",
    category: "control", tags: ["uiverse", "加载", "网格", "呼吸", "css"],
    prompt: "请把「方块呼吸加载器」装进我的项目：一个加载指示器——蓝色细描边圆角方框内是 2×2 四个同色方块，四个方块各错开 200ms 依次在 1s 周期里横向/纵向伸张（flex 1→4→1），整体像呼吸一样此起彼伏；外框尺寸 158px 上限、内边距与间隙 4px。容器加 role=\"status\" + 视觉隐藏的「加载中」文案，尊重 prefers-reduced-motion（改为静态或轻微透明度呼吸）。先看现有加载组件，融入而不是覆盖。" },
  { file: "loaders/bociKond_foolish-sloth-24.html", id: "bar-sweep-loader", title: "扫条加载器", titleEn: "Bar Sweep Loader",
    desc: "描边胶囊里色块从左扫到右的进度感加载器。", descEn: "Outlined pill loader swept left to right by a color bar.",
    tags: ["uiverse", "加载", "进度", "极简", "css"],
    prompt: "请把「扫条加载器」装进我的项目：一个极简加载条——胶囊形容器只有 5px 描边（outline + outline-offset，无背景），内部一条同色实心块以 2s ease-in-out 无限循环从左铺满到右再重来（width 0%→100%）；颜色与时长用 CSS 变量（--clr / --load-time）暴露，方便主题接线；提供竖排选项（rotate: -90deg）。加 role=\"progressbar\"（不定态，aria-valuetext=\"加载中\"），prefers-reduced-motion 时改为透明度脉冲。先看现有加载/进度组件，融入而不是覆盖。" },
  { file: "loaders/AHMED-MIT_dry-wolverine-85.html", id: "dual-ring-loader", title: "双环互扣加载器", titleEn: "Dual Ring Loader",
    desc: "两个缺口圆环反向旋转互扣的加载器。", descEn: "Two notched rings rotating in opposite directions.",
    category: "control", tags: ["uiverse", "加载", "圆环", "极简", "css"],
    prompt: "请把「双环互扣加载器」装进我的项目：一个加载指示器——两个只有描边（无缺口用透明弧段作视觉断点）的圆环叠放，各自以 1s 线性匀速反向旋转（一个顺时针一个逆时针），交错出互扣的动感；环径与线宽用 CSS 变量暴露，颜色取 currentColor 便于放进任何容器。加 role=\"status\" + 视觉隐藏文案，prefers-reduced-motion 时改为静态双环。先看现有加载组件，融入而不是覆盖。" },
  { file: "Toggle-switches/Creatlydev_wicked-bear-73.html", id: "day-night-letter-toggle", title: "昼夜字母开关", titleEn: "Day-Night Letter Toggle",
    desc: "左右字母 D/G 之间，圆点滑块滑动的开关。", descEn: "Toggle flanked by letters with a sliding dot knob.",
    tags: ["uiverse", "开关", "昼夜", "主题切换", "css"],
    prompt: "请把「昼夜字母开关」装进我的项目：一个主题开关——左右两个超大粗体字母（D 与 G，代表 Day/Glow 或自定义），中间是白色胶囊轨道，内藏一枚深色圆点；切换时圆点以 400ms ease-in-out 平滑滑到另一端，轨道底色与圆点颜色互换（500ms/1000ms 的差异化时长让颜色比位移慢半拍），并在暗态叠加一层大范围同色泛光（box-shadow 40px 级）；状态由真 checkbox 承载（视觉隐藏），label 包裹整块，focus-visible 有描边。先看现有开关/主题切换控件，融入而不是覆盖。" },
  { file: "Toggle-switches/gharsh11032000_green-liger-89.html", id: "ios-style-switch", title: "iOS 风格开关", titleEn: "iOS Style Switch",
    desc: "苹果风开关：滑块放大带阴影、开启转绿。", descEn: "Apple-style switch whose knob grows and turns track green.",
    tags: ["uiverse", "开关", "ios", "苹果风", "css"],
    prompt: "请把「iOS 风格开关」装进我的项目：一颗 iOS 风格开关——3.5em×2em 胶囊轨道，未选中是浅灰底带 1px 内阴影，滑块是白色圆点带投影停在左侧；选中时轨道转绿（约 #34C759 系），滑块平滑移到右侧并轻微放大（1.04，阴影加深）；轨道内加一层同色渐变让顶部略亮；尺寸随 font-size 缩放（全 em）。用真 checkbox（视觉隐藏）+ label 承载，键盘可达、focus-visible 有晕圈。先看现有开关控件，融入而不是覆盖。" },
  { file: "Toggle-switches/csemszepp_horrible-mouse-59.html", id: "gooey-color-switch", title: "黏滑变色开关", titleEn: "Gooey Color Switch",
    desc: "Gooey 滤镜做的开关：圆点黏糊地涨过去，红绿变色。", descEn: "Gooey-filtered switch where the knob stretches across and recolors.",
    category: "control", tags: ["uiverse", "开关", "gooey", "滤镜", "创意"],
    prompt: "请把「黏滑变色开关」装进我的项目：一颗创意开关——红色胶囊轨道（aspect-ratio 2.25，尺寸由 --s 变量控制）内一条白色层，圆点的「滑动」不是位移而是白色层的 radial-gradient 背景位与 padding 同步变化，再叠 mix-blend-mode:darken + blur/contrast 滤镜，做出圆点被拉长黏过去的 gooey 观感；选中时轨道转绿、圆点回到另一侧；整段过渡 0.3~0.4s，内边距用 cubic-bezier 超调回弹。用真 checkbox（appearance:none 直接样式化）承载，键盘可达。先看现有开关控件，融入而不是覆盖。" },
  { file: "Checkboxes/andrew-demchenk0_clever-cobra-93.html", id: "thumb-checkbox", title: "拇指赞踩复选框", titleEn: "Thumb Checkbox",
    desc: "一个拇指 SVG：勾选时翻转朝下并变红（赞→踩）。", descEn: "A thumb glyph that flips down and turns red when checked.",
    category: "control", tags: ["uiverse", "复选框", "图标", "点赞", "svg"],
    prompt: "请把「拇指赞踩复选框」装进我的项目：一个点赞/踩的图标复选框——默认是一枚蓝色拇指朝上的实心 SVG，勾选后拇指绕自身翻转朝下并转成红色（fill 过渡），点击有轻微缩放回弹；真 checkbox 视觉隐藏（opacity 0 / 零尺寸）但保留可聚焦，`:focus-visible + svg` 给描边圈；整块是 label 因而点击区域够大，带 aria-label 说明当前语义（赞/踩）。先看现有图标切换控件，融入而不是覆盖。" },
  { file: "Checkboxes/adamgiebl_polite-tiger-12.html", id: "classic-checkbox", title: "经典对勾复选框", titleEn: "Classic Checkbox",
    desc: "最经典的方框对勾复选框，勾选变蓝出白勾。", descEn: "The classic square checkbox that turns blue with a white tick.",
    category: "control", tags: ["uiverse", "复选框", "对勾", "表单", "css"],
    prompt: "请把「经典对勾复选框」装进我的项目：一颗经典复选框——20~25px 灰底方角方块，勾选后底色变蓝（#2196F3）并用 ::after 画白色对勾（右边框+下边框旋转 45°，勾出现时无过渡或轻微缩放）；真 checkbox 隐藏，label 包裹并可带文字说明；hover 底色略提亮，focus-visible 有外圈描边；带 disabled 态（降透明度、不响应点击）。先看现有表单控件，融入而不是覆盖。" },
  { file: "Tooltips/SteveBloX_orange-fox-41.html", id: "material-tooltip", title: "Material 提示气泡", titleEn: "Material Tooltip",
    desc: "Material Design 3 风格提示：悬停从上方浮出带标题的卡。", descEn: "Material 3 tooltip card with a title that floats in above.",
    category: "control", tags: ["uiverse", "tooltip", "material", "md3", "悬停"],
    prompt: "请把「Material 提示气泡」装进我的项目：一个 Material Design 3 风格 tooltip——触发文字为带小圆角胶囊底的粗体标签（浅紫底深紫字）；悬停/聚焦时上方浮出一张卡（浅紫底、圆角、双层阴影、带小箭头或缺口），卡内是标题（加粗）+ 两三行正文；浮出动画为透明度 + 轻微上移（Material 的 emphasized 缓动）；气泡 role=\"tooltip\" 并与触发元素 aria-describedby 关联，键盘聚焦同样可唤出，Esc 可关。先看现有 tooltip 体系，融入而不是覆盖。" },
  { file: "Notifications/jyefu013_unlucky-bird-53.html", id: "swap-text-notification", title: "换字提示按钮", titleEn: "Swap Text Notification",
    desc: "悬停时提示文字被放大换掉的成就通知按钮。", descEn: "Achievement toast whose label is scaled away for a new one on hover.",
    category: "control", tags: ["uiverse", "通知", "徽章", "悬停", "成就"],
    prompt: "请把「换字提示按钮」装进我的项目：一枚成就通知按钮——黑底、黄绿色文字的圆角标签（如「New Level Unlocked!」）；悬停时原来的文字从中心放大并淡出（scale 到 4 倍同时透明度归零，像被推近镜头），同时另一句短语从下方弹入居中显示（如「Congrats!」），离开时反向回收；两段文字绝对定位叠放，容器 overflow:hidden 裁掉溢出；按钮是真 button 语义、focus-visible 有描边，深色模式下黄绿文字保持对比。先看现有 toast/通知组件，融入而不是覆盖。" },
  { file: "Cards/alexruix_new-newt-64.html", id: "slide-reveal-card", title: "侧滑简介卡", titleEn: "Slide Reveal Card",
    desc: "悬停时正面文字滑出、背面简介滑入的渐变卡。", descEn: "Gradient card whose front slides away to reveal a back blurb.",
    category: "block", tags: ["uiverse", "卡片", "悬停", "侧滑", "渐变"],
    prompt: "请把「侧滑简介卡」装进我的项目：一张个人简介卡（约 190×254）——暖橙到黄的竖向渐变底 + 6px 浅色描边圆角；正面底部居中放姓名与职位（姓名下方一条 50% 宽的小圆角分隔条）；悬停时正面整体向左滑出（translateX -100%），背面正文以同一组 cubic-bezier(0.785,0.135,0.15,0.86) 缓动从右侧滑入（translateX 120%→0）；两张卡面都是绝对定位叠放、容器裁切，过渡 1s 带一点点先慢后快的电影感；触屏设备上点按同样触发（用 :focus-within 兜底），容器 role 语义与键盘焦点可见。先看现有卡片组件，融入而不是覆盖。" },
];

for (const entry of UIVERSE_COMPONENTS) {
  const raw = readSource(path.join(GALAXY, entry.file));
  // 署名行形如 `/* From Uiverse.io by 0x-Sarthak  - Tags: ... */`；用户名可含连字符，
  // 所以按空白切而不是按连字符（早先按 [^\s-]+ 会把 0x-Sarthak 截成 0x）。
  const author = /From Uiverse\.io by\s+([^\s]+)/.exec(raw)?.[1] ?? "未知";
  const id = `uiverse-${entry.id}`;
  const sourceUrl = `https://github.com/uiverse-io/galaxy/blob/main/${encodeURI(entry.file)}`;
  const header = `/**
 * 「${entry.title}」逐字收录（V3-3 素材库）。
 *
 * 来源：UIverse · github.com/uiverse-io/galaxy · ${entry.file}
 * 作者：${author}（UIverse.io 社区投稿）
 * 许可：CC BY 4.0（uiverse.io 站点条款；仓库 LICENSE 另标 MIT）——按 CC BY 要求
 *       逐件标注原作者与 UIverse.io 出处（见 source 字段与文件头的声明块）。
 * 文件为上游片段的逐字收录（原署名注释保留在 <style> 顶部）。
 */`;
  const body = `{
  id: ${ts(id)},
  title: ${ts(entry.title)},
  titleEn: ${ts(entry.titleEn)},
  description: ${ts(entry.desc)},
  category: ${ts(entry.category ?? "control")},
  tags: ${ts(entry.tags)},
  previewHtml: ${ts(renderUiversePreview(entry, raw, author))},
  files: [{ name: ${ts(`${entry.id}.html`)}, language: "html", content: ${ts(
    licenseHeader(
      {
        title: `${entry.title} · ${entry.id}.html`,
        file: entry.file,
        author: `${author}（UIverse.io 社区）`,
        copyright: `Copyright (c) ${author}`,
        license: "CC BY 4.0（uiverse.io 站点条款；仓库 LICENSE 另标 MIT）",
        repo: "github.com/uiverse-io/galaxy",
        sourceUrl,
        extra: ["署名要求：CC BY 4.0 要求署名原作者与 UIverse.io。"],
      },
      "html",
    ) + raw,
  )} }],
  prompt: ${ts(entry.prompt)},
  source: { site: "UIverse", url: ${ts(sourceUrl)}, license: ${ts(`CC BY 4.0（作者 ${author} · UIverse.io）`)} },
}`;
  writeCard(`${id}.ts`, id, header, body);
}

/**
 * UIverse 预览页：上游片段是「标记 + <style>」，直接进 iframe 不居中也不撑满，
 * 这里套一层舞台（居中 + 深色底 + 提示行），片段本身逐字保留。
 */
function renderUiversePreview(entry, raw, author) {
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${entry.title}</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  html, body { height: 100%; }
  body {
    display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
    color: #cdd8ea; overflow: hidden;
  }
  .uv-stage { display: grid; place-items: center; gap: 14px; padding: 16px; }
  .uv-credit { font-size: 11px; letter-spacing: 0.06em; color: #5b6b8c; user-select: none; }
</style>
</head>
<body>
  <div class="uv-stage">
${raw.trim()}
    <p class="uv-credit">UIverse · ${author} · CC BY 4.0</p>
  </div>
</body>
</html>`;
}

console.log(`[ok] 写入 ${written.length} 张收录卡到 ${outDir}`);
// index 注册片段：PRINT_INDEX=1 时打印（片段供人手贴进 catalog/index.ts，脚本不代改）
if (process.env.PRINT_INDEX === "1") {
  console.log("--- index 注册片段 ---");
  for (const entry of written) {
    console.log(`import { ${entry.varName} } from "./assets/${entry.file.replace(/\.ts$/, ".js")}";`);
  }
  console.log("const OPEN_SOURCE_ASSETS: AssetManifest[] = [");
  for (const entry of written) console.log(`  ${entry.varName},`);
  console.log("];");
}
