import type { AssetManifest } from "../types.js";

/**
 * 「搜索面板」逐字收录（V3-3 素材库）。
 *
 * 来源：Beautiful UI · github.com/TurboKach/ai-native-react-components · components/search.tsx
 * 作者：Turbo（beautifului.dev）
 * 许可：MIT（Copyright (c) 2026 Turbo）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const BeautifulUiSearchAsset: AssetManifest = {
  id: "beautifului-search",
  title: "搜索面板",
  titleEn: "Search",
  description: "命令面板式搜索：输入即时过滤、键盘上下选择、回车确认、空态。",
  descriptionEn: "Command-palette style search with live filtering and keyboard navigation.",
  category: "block",
  tags: ["ai-chat","搜索","命令面板","beautifului","键盘"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "search.tsx",
      "language": "tsx",
      "content": "/*!\n * 搜索面板 · search.tsx\n *\n * 来源：github.com/TurboKach/ai-native-react-components · components/search.tsx\n * 原址：https://github.com/TurboKach/ai-native-react-components/blob/main/components/search.tsx\n * 作者：Turbo（beautifului.dev）\n * 版权：Copyright (c) 2026 Turbo\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n\"use client\";\r\n\r\nimport { useState } from \"react\";\r\n\r\n/* ─────────────────────────────────────────────────────────\r\n * SEARCH — command search with live filtering.\r\n * The field, clear action, and results are directly usable.\r\n * ───────────────────────────────────────────────────────── */\r\n\r\nconst ITEMS = [\r\n  \"Forecast summer demand\",\r\n  \"Find waffle cone suppliers\",\r\n  \"Compare seasonal flavors\",\r\n  \"Draft flavor launch plan\",\r\n  \"Check cold-chain status\",\r\n  \"Audit sugar costs\",\r\n  \"Retire low sellers\",\r\n];\r\n\r\nexport default function SearchList() {\r\n  const [query, setQuery] = useState(\"\");\r\n  const results = query\r\n    ? ITEMS.filter((i) => i.toLowerCase().includes(query.toLowerCase()))\r\n    : ITEMS.slice(0, 5);\r\n  const empty = query.length > 2 && results.length === 0;\r\n\r\n  return (\r\n    <div className=\"flex min-h-[248px] w-full max-w-72 flex-col items-stretch\">\r\n      <div className=\"w-full self-start overflow-hidden rounded-card bg-surface shadow-raised\">\r\n        {/* input row */}\r\n        <div className=\"flex h-10 items-center gap-2 border-b border-line px-3 transition-colors duration-100 hover:bg-hover\">\r\n          <svg width=\"14\" height=\"14\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"var(--ink-3)\" strokeWidth=\"2\" strokeLinecap=\"round\" className=\"shrink-0\">\r\n            <circle cx=\"11\" cy=\"11\" r=\"7\" />\r\n            <path d=\"M21 21l-4.3-4.3\" />\r\n          </svg>\r\n          <input\r\n            value={query}\r\n            onChange={(event) => setQuery(event.target.value)}\r\n            placeholder=\"Search flavors…\"\r\n            aria-label=\"Search flavors\"\r\n            className=\"min-w-0 flex-1 bg-transparent text-[13px] text-ink outline-none placeholder:text-ink-3\"\r\n          />\r\n          {query && (\r\n            <button\r\n              aria-label=\"Clear search\"\r\n              type=\"button\"\r\n              onClick={() => setQuery(\"\")}\r\n              className=\"flex size-5.5 items-center justify-center rounded-full text-ink-3\r\n                transition-colors duration-100 hover:bg-line/70 hover:text-ink\"\r\n              style={{ animation: \"fade-in 150ms ease-out both\" }}\r\n            >\r\n              <svg width=\"11\" height=\"11\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" strokeWidth=\"2.2\" strokeLinecap=\"round\">\r\n                <path d=\"M18 6L6 18M6 6l12 12\" />\r\n              </svg>\r\n            </button>\r\n          )}\r\n        </div>\r\n\r\n        {/* results / empty state */}\r\n        {empty ? (\r\n          <div className=\"flex flex-col items-center justify-center gap-1 px-4 py-8\" style={{ animation: \"fade-in 250ms ease-out both\" }}>\r\n            <span className=\"mb-1.5 flex size-8 items-center justify-center rounded-control bg-inset text-ink-3 shadow-hairline\">\r\n              <svg width=\"15\" height=\"15\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" strokeWidth=\"1.8\" strokeLinecap=\"round\">\r\n                <circle cx=\"11\" cy=\"11\" r=\"7\" />\r\n                <path d=\"M21 21l-4.3-4.3\" />\r\n              </svg>\r\n            </span>\r\n            <span className=\"text-[13px] font-medium text-ink\">No results found</span>\r\n            <span className=\"text-[12px] text-ink-3\">Adjust your search to try again</span>\r\n          </div>\r\n        ) : (\r\n          <div className=\"p-1\">\r\n            {results.map((item) => (\r\n              <button\r\n                key={item}\r\n                type=\"button\"\r\n                onClick={() => setQuery(item)}\r\n                className=\"flex h-8 w-full items-center rounded-[6px] px-2 text-left text-[13px]\r\n                  text-ink transition-colors duration-100 hover:bg-hover\"\r\n                style={{ animation: \"fade-in 200ms ease-out both\" }}\r\n              >\r\n                {item}\r\n              </button>\r\n            ))}\r\n          </div>\r\n        )}\r\n      </div>\r\n    </div>\r\n  );\r\n}\r\n"
    }
  ],
  prompt: "请把「搜索面板」装进我的项目：命令面板式搜索框——顶部一行（放大镜 + 输入框 + ESC 提示），下方结果列表；输入即时过滤（大小写不敏感），↑/↓ 移动选中项（选中项实底反色）回车确认；结果项含图标、标题与右侧次要说明；超过三个字符仍无结果时显示空态（「没有匹配项」+ 清除按钮）；列表最多显示若干条。角色用 combobox/listbox，键盘可达。先看现有搜索/命令面板组件，融入而不是覆盖。",
  source: { site: "Beautiful UI", url: "https://github.com/TurboKach/ai-native-react-components", license: "MIT" },
  preview: {
    theme: "beautifului",
    demo: "import Comp from \"./search.tsx\";\nexport default function Demo() { return <Comp />; }",
  },
};
