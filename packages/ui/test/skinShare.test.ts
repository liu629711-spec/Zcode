import assert from "node:assert/strict";
import test from "node:test";
import { validateSkin } from "../src/skin-engine/skinSchema.js";
import {
  buildSkinShareUrl,
  decodeSkinFromShare,
  encodeSkinForShare,
} from "../src/skin-engine/skinShare.js";

function skinOf(raw: Record<string, unknown>) {
  const result = validateSkin({ formatVersion: 1, id: "share-test", name: "分享测试", ...raw });
  assert.equal(result.ok, true);
  return result.ok ? result.skin : undefined;
}

test("分享链接：纯配色皮肤编码→解码往返一致（含中文作者名）", () => {
  const skin = skinOf({
    author: "老板",
    light: { background: "#f5f4ed", brand: "#c96442" },
    dark: { ink: "#faf9f5" },
    glass: { enabled: true, blend: 0.4, sidebar: 0.5, panel: 0.4, composer: 0.3 },
  });
  const encoded = encodeSkinForShare(skin);
  assert.ok(encoded);
  assert.ok(!encoded.includes("+") && !encoded.includes("/") && !encoded.includes("="));
  const decoded = decodeSkinFromShare(encoded);
  assert.equal(decoded.ok, true);
  if (decoded.ok) assert.deepEqual(decoded.skin, skin);
});

test("分享链接：带壁纸的皮肤拒绝进链接（URL 会爆长）", () => {
  const skin = skinOf({
    wallpaper: { kind: "gradient", gradient: "linear-gradient(0deg, red, blue)", opacity: 1, blurPx: 0, dim: 0 },
  });
  assert.equal(encodeSkinForShare(skin), null);
  // http 图片源同样走文件分享
  const withUrl = skinOf({
    wallpaper: { kind: "image", imageDataUrl: "https://example.com/a.png", opacity: 1, blurPx: 0, dim: 0 },
  });
  assert.equal(encodeSkinForShare(withUrl), null);
});

test("分享链接：截断/篡改的参数解析失败且不炸", () => {
  const encoded = encodeSkinForShare(skinOf({ light: { brand: "#c96442" } }));
  assert.ok(encoded);
  const truncated = decodeSkinFromShare(encoded.slice(0, Math.floor(encoded.length / 2)));
  assert.equal(truncated.ok, false);
  assert.ok(truncated.errors.length > 0);
});

test("分享链接：buildSkinShareUrl 拼 hash（node 环境无 location 返回 null）", () => {
  const skin = skinOf({ light: { brand: "#c96442" } });
  // node:test 环境没有 location，这里只验证不炸
  assert.equal(buildSkinShareUrl(skin), null);
});
