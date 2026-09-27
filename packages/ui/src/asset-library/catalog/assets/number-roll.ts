import type { AssetManifest } from "../types.js";

/**
 * 数字滚动计数器（react 类种子货，自制原创）：每位数字是一条 0-9 竖向胶片，
 * 变号时整条胶片 translateY 滑到对应格，带 overshoot 回弹。
 *
 * react 货约定：files[0] 是 tsx 图纸（完整可复制源码）；previewHtml 不在本文件写，
 * 由 catalog/index.ts 从 preview-html.ts 的 REACT_PREVIEW_HTML 接线（构建期
 * scripts/build-asset-previews.mjs 生成）。demo 源码避免反引号，嵌入模板串不用转义。
 */

const DEMO_TSX = `import { useEffect, useState } from "react";

// 每位数字一条 0-9 竖向胶片：变号时整条滑到对应格，overshoot 贝塞尔带回弹。
const DIGITS = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

const CSS = [
  "* { box-sizing: border-box; margin: 0; }",
  "body { min-height: 100vh; display: grid; place-items: center; background: #0b101a; font-family: system-ui, 'PingFang SC', 'Microsoft YaHei', sans-serif; }",
  ".nr-stage { display: flex; flex-direction: column; align-items: center; gap: 18px; color: #e6edf7; }",
  ".nr-counter { display: flex; gap: 6px; padding: 18px 22px; border-radius: 16px; background: #141c2b; box-shadow: inset 0 2px 10px rgb(0 0 0 / 0.5); }",
  ".nr-window { display: inline-block; height: 64px; overflow: hidden; border-radius: 8px; background: #0d1420; }",
  ".nr-strip { display: flex; flex-direction: column; transition: transform 0.7s cubic-bezier(0.22, 1.6, 0.36, 1); }",
  ".nr-cell { height: 64px; width: 44px; line-height: 64px; text-align: center; font-size: 40px; font-variant-numeric: tabular-nums; }",
  ".nr-hint { font-size: 13px; color: #8ea0bd; }",
  "@media (prefers-reduced-motion: reduce) { .nr-strip { transition: none; } }",
].join("\\n");

export default function NumberRoll() {
  const [value, setValue] = useState(2026);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setValue(Math.floor(1000 + Math.random() * 9000));
    }, 2200);
    return () => window.clearInterval(timer);
  }, []);

  const digits = String(value).split("");
  return (
    <div className="nr-stage">
      <style>{CSS}</style>
      <div className="nr-counter" role="img" aria-label={"当前数值 " + value}>
        {digits.map((digit, index) => (
          <span className="nr-window" key={index}>
            <span
              className="nr-strip"
              style={{ transform: "translateY(-" + Number(digit) * 10 + "%)" }}
            >
              {DIGITS.map((d) => (
                <span className="nr-cell" key={d}>
                  {d}
                </span>
              ))}
            </span>
          </span>
        ))}
      </div>
      <p className="nr-hint">数字滚动计数器 · 每 2.2 秒跳一个四位数</p>
    </div>
  );
}
`;

export const numberRollAsset: AssetManifest = {
  id: "number-roll",
  title: "数字滚动计数器",
  description: "React 数字滚动：逐位竖向胶片滑到目标数，overshoot 回弹像老虎机。",
  category: "text-animation",
  tags: ["react", "数字动效", "计数器", "暗色"],
  // 由 catalog/index.ts 从 REACT_PREVIEW_HTML["number-roll"] 接线（此处占位空串）
  previewHtml: "",
  files: [{ name: "NumberRoll.tsx", language: "tsx", content: DEMO_TSX }],
  prompt:
    "请把「数字滚动计数器」装进我的项目：一个 React 计数器组件，数值变化时每位数字以竖向 0-9 胶片形式滑到目标格（translateY + overshoot 贝塞尔回弹），等宽数字避免跳动；尊重 prefers-reduced-motion。先看现有数字/统计展示组件的风格，融入而不是覆盖。",
};
