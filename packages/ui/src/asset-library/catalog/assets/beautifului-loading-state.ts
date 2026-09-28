import type { AssetManifest } from "../types.js";

/**
 * 「加载态（像素网格）」逐字收录（V3-3 素材库）。
 *
 * 来源：Beautiful UI · github.com/TurboKach/ai-native-react-components · components/loading-state.tsx
 * 作者：Turbo（beautifului.dev）
 * 许可：MIT（Copyright (c) 2026 Turbo）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const BeautifulUiLoadingStateAsset: AssetManifest = {
  id: "beautifului-loading-state",
  title: "加载态（像素网格）",
  titleEn: "Loading State",
  description: "长任务的像素网格加载器：雪佛龙波前推进 + 流光文案 + 实时计时。",
  descriptionEn: "Pixel-grid loader with a shimmering label and a live elapsed timer.",
  category: "block",
  tags: ["ai-chat","加载","等待","beautifului","像素"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "loading-state.tsx",
      "language": "tsx",
      "content": "/*!\n * 加载态（像素网格） · loading-state.tsx\n *\n * 来源：github.com/TurboKach/ai-native-react-components · components/loading-state.tsx\n * 原址：https://github.com/TurboKach/ai-native-react-components/blob/main/components/loading-state.tsx\n * 作者：Turbo（beautifului.dev）\n * 版权：Copyright (c) 2026 Turbo\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n\"use client\";\r\n\r\nimport { useEffect, useState } from \"react\";\r\n\r\n/* ─────────────────────────────────────────────────────────\r\n * LOADING STATE — pixel-grid loader for long-running work\r\n *\r\n * Variants:\r\n *   Drive  — square cells, chevron wavefront driving right;\r\n *            the 650ms cycle is shorter than the sweep, so\r\n *            two fronts are always in flight\r\n *   Dots   — same wavefront, circular cells\r\n *   Orbit  — a comet lapping the grid perimeter\r\n *\r\n * Paired with a shimmering label and a live elapsed timer\r\n * in mono tabular figures. Reduced motion freezes the grid\r\n * to its dim state; the timer still ticks.\r\n * ───────────────────────────────────────────────────────── */\r\n\r\nconst chevron = Array.from({ length: 9 }, (_, i) => {\r\n  const r = Math.floor(i / 3), c = i % 3;\r\n  return (c + Math.abs(r - 1)) * 90;\r\n});\r\n\r\nconst ORBIT_ORDER = [0, 1, 2, 5, 8, 7, 6, 3];\r\nconst orbit = Array.from({ length: 9 }, (_, i) => {\r\n  const k = ORBIT_ORDER.indexOf(i);\r\n  return k === -1 ? null : k * 110;\r\n});\r\n\r\nconst PATTERNS: Record<string, { delays: (number | null)[]; dur: number; round: boolean }> = {\r\n  Drive: { delays: chevron, dur: 650, round: false },\r\n  Dots: { delays: chevron, dur: 650, round: true },\r\n  Orbit: { delays: orbit, dur: 950, round: false },\r\n};\r\n\r\nfunction useElapsed() {\r\n  const [ds, setDs] = useState(0);\r\n  useEffect(() => {\r\n    const t = setInterval(() => setDs((d) => d + 1), 100);\r\n    return () => clearInterval(t);\r\n  }, []);\r\n  const total = ds / 10;\r\n  if (total < 60) return `${total.toFixed(1)}s`;\r\n  return `${Math.floor(total / 60)}m ${(total % 60).toFixed(1)}s`;\r\n}\r\n\r\nexport default function LoadingState({\r\n  label = \"Churning\",\r\n  variant = \"Drive\",\r\n}: {\r\n  label?: string;\r\n  variant?: string;\r\n}) {\r\n  const elapsed = useElapsed();\r\n  const { delays, dur, round } = PATTERNS[variant] ?? PATTERNS.Drive;\r\n\r\n  return (\r\n    <div className=\"flex w-fit items-center gap-2.5\">\r\n      <span aria-hidden className=\"grid grid-cols-[repeat(3,4px)] gap-[1.5px]\">\r\n        {delays.map((d, i) => (\r\n          <span\r\n            key={i}\r\n            className={`size-[4px] bg-ink ${round ? \"rounded-full\" : \"rounded-[1px]\"}`}\r\n            style={{\r\n              opacity: d === null ? 0.07 : 0.15,\r\n              animation:\r\n                d === null ? \"none\" : `pixel-on ${dur}ms ease-in-out ${d}ms infinite`,\r\n            }}\r\n          />\r\n        ))}\r\n      </span>\r\n      <span\r\n        className=\"bg-clip-text text-[13px] font-medium text-transparent\"\r\n        style={{\r\n          backgroundImage:\r\n            \"linear-gradient(90deg, var(--ink-3) 35%, var(--ink) 50%, var(--ink-3) 65%)\",\r\n          backgroundSize: \"200% 100%\",\r\n          animation: \"shimmer-text 1.4s linear infinite\",\r\n        }}\r\n      >\r\n        {label}\r\n      </span>\r\n      <span className=\"font-mono text-[12px] text-ink-3 tabular-nums\">\r\n        {elapsed}\r\n      </span>\r\n    </div>\r\n  );\r\n}\r\n"
    }
  ],
  prompt: "请把「像素网格加载态」装进我的项目：智能体长时间干活时的加载指示器——3×3 像素网格按雪佛龙波前（chevron wavefront）错位点亮循环推进，周期短于横扫周期因而看起来永远有两道波在飞；旁边是流光扫过的文字标签（Shimmer，light sweep 沿字形滑过而不是整块闪）；跟随一个等宽 tabular 数字的实时耗时计时（100ms 一跳，超过 60 秒转 m/s 格式）。三种变体：Drive（方格）/Dots（圆点）/Orbit（彗星绕圈，按 perimeter 顺序点亮）。尊重 prefers-reduced-motion：网格冻结在暗态但计时继续走。先看现有的加载/骨架组件，融入而不是覆盖。",
  source: { site: "Beautiful UI", url: "https://github.com/TurboKach/ai-native-react-components", license: "MIT" },
  preview: {
    theme: "beautifului",
    demo: "import Comp from \"./loading-state\";\nexport default function Demo() { return <Comp />; }",
  },
};
