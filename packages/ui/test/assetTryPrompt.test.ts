import assert from "node:assert/strict";
import test from "node:test";
import { buildAssetTryPrompt } from "../src/asset-library/assetTryPrompt.js";
import { ASSET_CATALOG } from "../src/asset-library/catalog/index.js";
import type { AssetManifest } from "../src/asset-library/catalog/types.js";

// ============================================================
// 递活管线组装的可运行检查（技术设计 §3/§6）
// ============================================================
// 钉的是：普通货=口令段+逐文件图纸段（文件名在围栏前一行）；图纸里出现 ```
// 时围栏逐文件加长（CommonMark 围栏规则）；prompt 类货=口令原文+固定尾句。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/assetTryPrompt.test.ts

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

test("普通货：口令段 + 图纸段 + 文件名进围栏前一行", () => {
  const out = buildAssetTryPrompt(manifest());
  assert.ok(out.startsWith("请把下面的「测试货」装进我的项目。\n"), out);
  assert.ok(out.includes("要求：把测试货装进项目"));
  assert.ok(out.includes("注意：先看现有框架和风格，融入而不是覆盖；装完告诉我改了哪些文件。"));
  assert.ok(out.includes("代码："));
  assert.ok(out.includes("Test.tsx\n```tsx\nexport const x = 1;\n```"), out);
});

test("图纸含 ```：围栏加长为四反引号且内容原样", () => {
  const content = "const md = `\n```js\nhi\n```\n`;";
  const out = buildAssetTryPrompt(manifest({ files: [{ name: "a.ts", language: "ts", content }] }));
  assert.ok(out.includes("\n````ts\n" + content + "\n````"), out);
  // 内容里的 ``` 不能充当关栏：整个图纸必须仍包在加长围栏里
  const lines = out.split("\n");
  assert.equal(lines.filter((line) => line === "````").length, 1);
  assert.ok(!lines.includes("```ts"), "不得出现会被内容破掉的普通围栏");
});

test("多文件多围栏：每个文件独立计算围栏长度", () => {
  const out = buildAssetTryPrompt(
    manifest({
      files: [
        { name: "plain.css", language: "css", content: ".a { color: red; }" },
        { name: "notes.md", language: "md", content: "```\nfenced\n```" },
      ],
    }),
  );
  assert.ok(out.includes("plain.css\n```css\n.a { color: red; }\n```"), out);
  assert.ok(out.includes("notes.md\n````md\n```\nfenced\n```\n````"), out);
});

test("prompt 类货：口令原文 + 固定尾句，无图纸段", () => {
  const out = buildAssetTryPrompt(
    manifest({ category: "prompt", files: [], prompt: "做一个毛玻璃登录页" }),
  );
  assert.equal(out, "做一个毛玻璃登录页\n\n请照上面的口令直接干活，不需要另附代码。");
});

test("真实目录冒烟：普通货三段齐全，prompt 货不带图纸段", () => {
  for (const asset of ASSET_CATALOG) {
    const out = buildAssetTryPrompt(asset);
    assert.ok(out.trim().length > 0, `${asset.id} 的递活消息不能为空`);
    if (asset.files.length === 0) {
      assert.ok(!out.includes("代码："), `${asset.id} 是 prompt 货，不该带图纸段`);
    } else {
      assert.ok(out.includes(`「${asset.title}」`) && out.includes("代码："), asset.id);
      for (const file of asset.files) {
        assert.ok(out.includes(`${file.name}\n`), `${asset.id} 缺文件名行 ${file.name}`);
      }
    }
  }
});
