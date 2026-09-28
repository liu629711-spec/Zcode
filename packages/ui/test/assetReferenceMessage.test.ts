import assert from "node:assert/strict";
import test from "node:test";
import { buildAssetReferenceChipMessage } from "../src/asset-library/assetReferenceMessage.js";
import { buildAssetTryPrompt } from "../src/asset-library/assetTryPrompt.js";
import type { AssetManifest } from "../src/asset-library/catalog/types.js";

// ============================================================
// 递活引用化组装的可运行检查（技术设计 §10 V2-2 → V3-2 chip 化 → V4.4 纯 chip 化）
// ============================================================
// 钉的是：chip 指向入口文件（index.html 优先）、label 是素材名而非文件名；
// V4.4 用户拍板：正文**只有 chip 本身**（composer 里呈现为一个"素材库引用"块），
// 不带口令/说明文字——需求描述由用户自己写。prompt 类货不落盘，
// buildAssetTryPrompt 仍保持原语义。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/assetReferenceMessage.test.ts

function manifest(overrides: Partial<AssetManifest> = {}): AssetManifest {
  return {
    id: "test-asset",
    title: "测试货",
    description: "测试用素材",
    category: "control",
    tags: ["test"],
    previewHtml: "<p>x</p>",
    files: [{ name: "Test.tsx", language: "tsx", content: "export const x = 1;" }],
    prompt: "把测试货装进项目",
    ...overrides,
  };
}

test("prompt 类货：不落盘不引用，buildAssetTryPrompt 保持原语义（口令原文+固定尾句）", () => {
  const promptAsset = manifest({ files: [], prompt: "给登录页加一条流星雨背景" });
  const out = buildAssetTryPrompt(promptAsset);
  assert.equal(out, "给登录页加一条流星雨背景\n\n请照上面的口令直接干活，不需要另附代码。");
  assert.ok(!out.includes(".zcode/asset-library"), "prompt 类货没有任何引用链接");
});

// ============================================================
// V4.4 纯 chip 化（用户反馈：只要"素材库引用"块，不要口令等文字）
// ============================================================

test("纯 chip：正文=mention.markdown 本身，无口令无落盘指引，mention 指向入口文件", () => {
  const asset = manifest({
    id: "chip-asset",
    title: "毛玻璃登录页",
    files: [
      { name: "style.css", language: "css", content: "body{}" },
      { name: "index.html", language: "html", content: "<p>x</p>" },
    ],
  });
  const paths = [
    "./.zcode/asset-library/chip-asset/style.css",
    "./.zcode/asset-library/chip-asset/index.html",
  ];
  const { text, mention } = buildAssetReferenceChipMessage(asset, paths, "zh-CN");
  assert.ok(mention, "应产出结构化 mention");
  assert.equal(mention!.category, "files");
  assert.equal(mention!.label, "毛玻璃登录页", "chip label 用素材名");
  assert.equal(mention!.value, "./.zcode/asset-library/chip-asset/index.html", "chip 指向入口文件");
  assert.equal(text, mention!.markdown, "正文就是 chip 本身（V4.4 纯引用块）");
  assert.ok(!text.includes("把测试货装进项目"), "不带口令（用户自己写说明）");
  assert.ok(!text.includes("图纸已写进项目"), "不带落盘指引");
  assert.ok(!text.includes("style.css"), "不含文件路径列表（路径由 chip 承载）");
});

test("纯 chip：无入口文件时取首个文件；路径列表为空则退回纯文本（不静默）", () => {
  const asset = manifest({
    id: "single",
    files: [{ name: "One.tsx", language: "tsx", content: "export {}" }],
  });
  const { mention } = buildAssetReferenceChipMessage(asset, ["./.zcode/asset-library/single/One.tsx"]);
  assert.equal(mention?.value, "./.zcode/asset-library/single/One.tsx");

  const fallback = buildAssetReferenceChipMessage(asset, []);
  assert.equal(fallback.mention, undefined);
  assert.ok(fallback.text.includes("One.tsx"), "空落盘退回逐文件引用文本");
});

test("纯 chip：英文 locale 用 titleEn 作 label", () => {
  const asset = manifest({ title: "中文名", titleEn: "English Name" });
  const { mention } = buildAssetReferenceChipMessage(
    asset,
    ["./.zcode/asset-library/test-asset/Test.tsx"],
    "en-US",
  );
  assert.equal(mention?.label, "English Name");
});

test("设计风格类：chip 后必须跟随口令（纯 chip 会让智能体零上下文，真机反馈）", () => {
  const asset = manifest({
    id: "design-agentic",
    title: "Agentic · 设计风格",
    category: "design-style",
    prompt: "请按「Agentic」这套设计风格改造我的界面。",
  });
  const { text, mention } = buildAssetReferenceChipMessage(
    asset,
    ["./.zcode/asset-library/design-agentic/DESIGN.md"],
    "zh-CN",
  );
  assert.ok(mention, "应产出结构化 mention");
  assert.ok(text.startsWith(mention!.markdown), "chip 开头");
  assert.ok(
    text.includes("请按「Agentic」这套设计风格改造我的界面。"),
    "设计风格 chip 后跟随口令",
  );
});
