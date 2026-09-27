import assert from "node:assert/strict";
import test from "node:test";
import { buildAssetReferenceMessage } from "../src/asset-library/assetReferenceMessage.js";
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
