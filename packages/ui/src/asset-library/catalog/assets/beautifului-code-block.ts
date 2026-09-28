import type { AssetManifest } from "../types.js";

/**
 * 「代码块」逐字收录（V3-3 素材库）。
 *
 * 来源：Beautiful UI · github.com/TurboKach/ai-native-react-components · components/code-block.tsx
 * 作者：Turbo（beautifului.dev）
 * 许可：MIT（Copyright (c) 2026 Turbo）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const BeautifulUiCodeBlockAsset: AssetManifest = {
  id: "beautifului-code-block",
  title: "代码块",
  titleEn: "Code Block",
  description: "智能体写码流式块：逐行流入、语法着色、复制按钮。",
  descriptionEn: "Agent code block streaming line by line with syntax colors and copy.",
  category: "block",
  tags: ["ai-chat","代码块","流式","beautifului","复制"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "code-block.tsx",
      "language": "tsx",
      "content": "/*!\n * 代码块 · code-block.tsx\n *\n * 来源：github.com/TurboKach/ai-native-react-components · components/code-block.tsx\n * 原址：https://github.com/TurboKach/ai-native-react-components/blob/main/components/code-block.tsx\n * 作者：Turbo（beautifului.dev）\n * 版权：Copyright (c) 2026 Turbo\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n\"use client\";\r\n\r\nimport { useCallback, useEffect, useState } from \"react\";\r\n\r\n/* ─────────────────────────────────────────────────────────\r\n * CODE BLOCK\r\n * Agent-written code streams line by line; copy is live.\r\n * ───────────────────────────────────────────────────────── */\r\n\r\nconst LINE_MS = 240;\r\nconst HOLD_MS = 3200;\r\n\r\ntype Tok = { t: string; c?: \"kw\" | \"str\" | \"num\" | \"fn\" | \"dim\" };\r\n\r\nconst LINES: Tok[][] = [\r\n  [{ t: \"export async function \", c: \"kw\" }, { t: \"churnBatch\", c: \"fn\" }, { t: \"() {\", c: \"dim\" }],\r\n  [{ t: \"  const \", c: \"kw\" }, { t: \"flavor = \" }, { t: \"await \", c: \"kw\" }, { t: \"getFlavor\", c: \"fn\" }, { t: \"(\", c: \"dim\" }, { t: \"\\\"pistachio\\\"\", c: \"str\" }, { t: \");\", c: \"dim\" }],\r\n  [{ t: \"  const \", c: \"kw\" }, { t: \"base = \" }, { t: \"await \", c: \"kw\" }, { t: \"dairy.\" }, { t: \"fetch\", c: \"fn\" }, { t: \"({ flavor });\", c: \"dim\" }],\r\n  [{ t: \"  await \", c: \"kw\" }, { t: \"freezer.\" }, { t: \"store\", c: \"fn\" }, { t: \"(base, { temp: \", c: \"dim\" }, { t: \"\\\"-14C\\\"\", c: \"str\" }, { t: \" });\", c: \"dim\" }],\r\n  [{ t: \"  return \", c: \"kw\" }, { t: \"base.gallons;\" }],\r\n  [{ t: \"}\", c: \"dim\" }],\r\n];\r\n\r\nconst COLORS: Record<string, string> = {\r\n  kw: \"var(--accent-ink)\",\r\n  str: \"var(--green)\",\r\n  num: \"var(--orange)\",\r\n  fn: \"var(--ink)\",\r\n  dim: \"var(--ink-3)\",\r\n};\r\n\r\nconst RAW = `export async function churnBatch() {\r\n  const flavor = await getFlavor(\"pistachio\");\r\n  const base = await dairy.fetch({ flavor });\r\n  await freezer.store(base, { temp: \"-14C\" });\r\n  return base.gallons;\r\n}`;\r\n\r\nexport default function CodeBlock() {\r\n  const [count, setCount] = useState(0);\r\n  const [copied, setCopied] = useState(false);\r\n  const done = count >= LINES.length;\r\n\r\n  useEffect(() => {\r\n    const t = setTimeout(\r\n      () => setCount((c) => (c >= LINES.length ? 0 : c + 1)),\r\n      count === 0 ? 400 : done ? HOLD_MS : LINE_MS,\r\n    );\r\n    return () => clearTimeout(t);\r\n  }, [count, done]);\r\n\r\n  const copy = useCallback(() => {\r\n    navigator.clipboard.writeText(RAW).then(() => {\r\n      setCopied(true);\r\n      setTimeout(() => setCopied(false), 1500);\r\n    });\r\n  }, []);\r\n\r\n  return (\r\n    <div className=\"w-full max-w-95 overflow-hidden rounded-card bg-surface shadow-card\">\r\n      {/* header */}\r\n      <div className=\"primitive-card-bar flex items-center justify-between border-b border-line\">\r\n        <span className=\"flex items-baseline gap-2\">\r\n          <span className=\"font-mono text-[12px] font-medium text-ink\">churn.ts</span>\r\n          <span className=\"text-[11.5px] text-ink-3\">TypeScript</span>\r\n        </span>\r\n        <button\r\n          aria-label=\"Copy code\"\r\n          onClick={copy}\r\n          className={`flex h-6 items-center gap-1 rounded-[6px] px-1.5 text-[11.5px]\r\n            font-medium transition-colors duration-100 hover:bg-hover\r\n            ${copied ? \"text-green\" : \"text-ink-3 hover:text-ink\"}`}\r\n        >\r\n          {copied ? (\r\n            <svg width=\"10\" height=\"10\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" strokeWidth=\"3\" strokeLinecap=\"round\" strokeLinejoin=\"round\"><path d=\"M20 6L9 17l-5-5\" /></svg>\r\n          ) : (\r\n            <svg width=\"10\" height=\"10\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" strokeWidth=\"2\" strokeLinecap=\"round\" strokeLinejoin=\"round\"><rect x=\"9\" y=\"9\" width=\"12\" height=\"12\" rx=\"2.5\" /><path d=\"M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1\" /></svg>\r\n          )}\r\n          {copied ? \"Copied\" : \"Copy\"}\r\n        </button>\r\n      </div>\r\n\r\n      {/* code */}\r\n      <pre className=\"min-h-[137px] bg-inset px-3 py-2.5 font-mono text-[11.5px] leading-[1.7]\">\r\n        {LINES.slice(0, count).map((line, i) => (\r\n          <div\r\n            key={i}\r\n            className=\"flex\"\r\n            style={{ animation: \"fade-up 250ms cubic-bezier(0.23,1,0.32,1) both\" }}\r\n          >\r\n            <span className=\"w-5 shrink-0 text-right text-[10.5px] leading-[1.86] text-ink-3/60 select-none\">\r\n              {i + 1}\r\n            </span>\r\n            <span className=\"pl-2.5 whitespace-pre\">\r\n              {line.map((tok, j) => (\r\n                <span key={j} style={{ color: tok.c ? COLORS[tok.c] : \"var(--ink-2)\" }}>\r\n                  {tok.t}\r\n                </span>\r\n              ))}\r\n              {i === count - 1 && !done && (\r\n                <span className=\"ml-0.5 inline-block h-3 w-[3px] translate-y-0.5 rounded-full bg-accent\" />\r\n              )}\r\n            </span>\r\n          </div>\r\n        ))}\r\n              </pre>\r\n    </div>\r\n  );\r\n}\r\n"
    }
  ],
  prompt: "请把「代码块」装进我的项目：智能体写代码时的流式代码块——代码逐行流出行出（每行 ~240ms，首行前稍等），关键字/字符串/函数名/注释四种着色，行号或左侧标记可选；顶部条含文件名与「复制」按钮（复制成功后短暂变「已复制」）；写完后停留数秒再重放。等宽字体，长行横向滚动不撑破卡片。先看现有代码块组件，融入而不是覆盖。",
  source: { site: "Beautiful UI", url: "https://github.com/TurboKach/ai-native-react-components", license: "MIT" },
  preview: {
    theme: "beautifului",
    demo: "import Comp from \"./code-block.tsx\";\nexport default function Demo() { return <Comp />; }",
  },
};
