import assert from "node:assert/strict";
import test from "node:test";
import {
  buildAssetReferenceChipMessage,
  buildAssetReferenceMessage,
} from "../src/asset-library/assetReferenceMessage.js";
import { buildAssetTryPrompt } from "../src/asset-library/assetTryPrompt.js";
import type { AssetManifest } from "../src/asset-library/catalog/types.js";

// ============================================================
// 递活引用化组装的可运行检查（技术设计 §10 V2-2）
// ============================================================
// 钉的是：消息=口令段+落盘说明句+逐文件 markdown 链接（./ 相对路径形态，
// 与 mentions/mentionMarkdown.ts 的文件引用语法一致）；prompt 类货不落盘，
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

test("单文件：口令 + 说明句 + 一条 ./ 相对路径引用链接", () => {
  const out = buildAssetReferenceMessage(manifest(), ["./.zcode/asset-library/test-asset/Test.tsx"]);
  assert.ok(out.startsWith("把测试货装进项目\n\n"), out);
  assert.ok(
    out.includes(
      "图纸已写进项目（1 个文件），直接读这些文件按口令装进项目，不要在回复里重复贴代码：",
    ),
    out,
  );
  assert.ok(
    out.endsWith("[Test.tsx](./.zcode/asset-library/test-asset/Test.tsx)"),
    out,
  );
  assert.ok(!out.includes("export const x = 1;"), "引用化消息不得再内联图纸代码");
});

test("多文件：按 writtenPaths 顺序逐文件一行链接，文件名与路径一一对应", () => {
  const out = buildAssetReferenceMessage(
    manifest({
      files: [
        { name: "index.html", language: "html", content: "<p>x</p>" },
        { name: "style.css", language: "css", content: ".a { color: red; }" },
      ],
    }),
    [
      "./.zcode/asset-library/test-asset/index.html",
      "./.zcode/asset-library/test-asset/style.css",
    ],
  );
  const linkLines = out.split("\n").filter((line) => line.startsWith("["));
  assert.deepEqual(linkLines, [
    "[index.html](./.zcode/asset-library/test-asset/index.html)",
    "[style.css](./.zcode/asset-library/test-asset/style.css)",
  ]);
  assert.ok(out.includes("（2 个文件）"), out);
});

test("prompt 类货：不落盘不引用，buildAssetTryPrompt 保持原语义（口令原文+固定尾句）", () => {
  const promptAsset = manifest({ files: [], prompt: "给登录页加一条流星雨背景" });
  const out = buildAssetTryPrompt(promptAsset);
  assert.equal(out, "给登录页加一条流星雨背景\n\n请照上面的口令直接干活，不需要另附代码。");
  assert.ok(!out.includes(".zcode/asset-library"), "prompt 类货没有任何引用链接");
});

// ============================================================
// V3-2 chip 化组装的可运行检查（用户反馈：要"引用 chip"不要代码/路径原文）
// ============================================================
// 钉的是：正文以 chip 的 canonical markdown 开头（composer 用 startsWith 判定结构化节点）、
// chip 指向入口文件（index.html 优先）、label 是素材名而非文件名、正文不含文件路径列表
// （路径由 chip 承载），且文本与 mention 一致成对返回。

test("chip 化：正文以引用 markdown 开头，mention 指向入口文件 index.html", () => {
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
  assert.ok(text.startsWith(mention!.markdown), "正文必须以 chip canonical markdown 开头");
  assert.ok(!text.includes("style.css"), "正文不含文件路径列表（路径由 chip 承载）");
  assert.ok(text.includes("把测试货装进项目"), "正文含口令");
});

test("chip 化：无入口文件时取首个文件；路径列表为空则退回纯文本（不静默）", () => {
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

test("chip 化：英文 locale 用 titleEn 作 label", () => {
  const asset = manifest({ title: "中文名", titleEn: "English Name" });
  const { mention } = buildAssetReferenceChipMessage(
    asset,
    ["./.zcode/asset-library/test-asset/Test.tsx"],
    "en-US",
  );
  assert.equal(mention?.label, "English Name");
});
