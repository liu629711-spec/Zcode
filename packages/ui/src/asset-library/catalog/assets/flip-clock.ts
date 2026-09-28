import type { AssetManifest } from "../types.js";

/**
 * 翻牌时钟（S5 批次 react 货，自制原创）：时:分:秒每位数字是一张立体卡片，
 * 数字变化时新值以 rotateX 翻落进场（key 重挂载重放动画），未变化位不动。
 *
 * react 货约定：files[0] 是 tsx 图纸（完整可复制源码）；previewHtml 不在本文件写，
 * 由 catalog/index.ts 从 preview-html.ts 的 REACT_PREVIEW_HTML 接线（构建期
 * scripts/build-asset-previews.mjs 生成）。demo 源码避免反引号，嵌入模板串不用转义。
 */

const DEMO_TSX = `import { useEffect, useState } from "react";

// 每位数字一张翻牌：值变化时 key 重挂载，rotateX 翻落进场；等宽数字防跳动。
const CSS = [
  "* { box-sizing: border-box; margin: 0; }",
  "body { min-height: 100%; display: grid; place-items: center; background: #0b0f17; font-family: ui-monospace, Consolas, 'PingFang SC', monospace; }",
  ".fc-stage { display: flex; flex-direction: column; align-items: center; gap: 16px; }",
  ".fc-clock { display: flex; gap: 8px; perspective: 640px; }",
  ".fc-digit { width: 46px; height: 68px; border-radius: 10px; display: grid; place-items: center; font-size: 38px; font-variant-numeric: tabular-nums; color: #e8eefb; background: linear-gradient(180deg, #1a2333, #111827); box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.08), 0 6px 14px rgb(0 0 0 / 0.45); }",
  ".fc-flip { animation: fc-flip 0.45s cubic-bezier(0.2, 0.9, 0.3, 1.2); transform-origin: 50% 0; }",
  ".fc-colon { width: 14px; display: grid; place-items: center; color: #64748b; font-size: 28px; }",
  ".fc-hint { font-size: 13px; color: #7c8db0; }",
  "@keyframes fc-flip { from { transform: rotateX(-80deg); opacity: 0.35; } to { transform: rotateX(0); opacity: 1; } }",
  "@media (prefers-reduced-motion: reduce) { .fc-flip { animation: none; } }",
].join("\\n");

function FlipDigit({ value }: { value: string }) {
  return (
    <span className="fc-digit">
      <span className="fc-flip" key={value}>
        {value}
      </span>
    </span>
  );
}

export default function FlipClock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  const hh = String(now.getHours()).padStart(2, "0");
  const mm = String(now.getMinutes()).padStart(2, "0");
  const ss = String(now.getSeconds()).padStart(2, "0");
  const cells = (hh + ":" + mm + ":" + ss).split("");
  return (
    <div className="fc-stage">
      <style>{CSS}</style>
      <div className="fc-clock" role="img" aria-label={"当前时间 " + hh + ":" + mm + ":" + ss}>
        {cells.map((ch, index) =>
          ch === ":" ? (
            <span className="fc-colon" key={"c" + index}>
              :
            </span>
          ) : (
            <FlipDigit key={index} value={ch} />
          ),
        )}
      </div>
      <p className="fc-hint">翻牌时钟 · 每秒走字，变化的数字翻落进场</p>
    </div>
  );
}
`;

export const flipClockAsset: AssetManifest = {
  id: "flip-clock",
  title: "翻牌时钟",
  titleEn: "Flip Clock",
  description: "React 实时时钟：走字的数字以立体翻牌落位。",
  descriptionEn: "A live React clock where changing digits flip into place.",
  category: "text-animation",
  tags: ["react", "时钟", "翻牌", "数字动效", "暗色"],
  // 由 catalog/index.ts 从 REACT_PREVIEW_HTML["flip-clock"] 接线（此处占位空串）
  previewHtml: "",
  files: [{ name: "FlipClock.tsx", language: "tsx", content: DEMO_TSX }],
  prompt:
    "请把「翻牌时钟」装进我的项目：一个 React 实时翻牌时钟组件——时:分:秒每位数字是一张立体卡片，数字变化时新值以 rotateX 翻落进场（带透视和轻微回弹），未变化的位保持静止；冒号分隔居中，等宽数字避免跳动；每秒走字用 setInterval 并在组件卸载时清理；保留时间语义的可访问性标签，尊重 prefers-reduced-motion（翻牌动画关闭）。先看现有的时间/状态展示组件风格，融入而不是覆盖。",
};
