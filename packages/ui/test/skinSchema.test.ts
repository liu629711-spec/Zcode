import assert from "node:assert/strict";
import test from "node:test";
import {
  createSkinFromStyle,
  serializeSkin,
  validateSkin,
  MAX_SKIN_IMAGE_CHARS,
} from "../src/skin-engine/skinSchema.js";

const baseSkin = {
  formatVersion: 1,
  id: "midnight-test",
  name: "深夜试装",
  baseStyle: "claude" as const,
};

test("校验：合法皮肤包通过且字段收敛", () => {
  const result = validateSkin({
    ...baseSkin,
    author: "老板",
    light: { background: "#F5F4ED", brand: "#C96442" },
    dark: { ink: "#faf9f5" },
    wallpaper: { kind: "gradient", gradient: "linear-gradient(135deg, #1a1c30, #242742)", opacity: 0.9, blurPx: 8, dim: 0.3 },
    glass: { enabled: true, blend: 0.4 },
    unknownFutureField: "新版客户端才认识的字段",
  });
  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.skin.light?.background, "#f5f4ed");
  assert.equal(result.skin.glass?.sidebar, 0.4);
  assert.equal(result.skin.glass?.panel, 0.4);
  assert.equal("unknownFutureField" in result.skin, false);
  assert.equal(result.skin.wallpaper?.dim, 0.3);
});

test("校验：focus-border 槽位不存在——机制级禁彩焦", () => {
  const result = validateSkin({
    ...baseSkin,
    light: { focusBorder: "#ff00ff" },
  });
  assert.equal(result.ok, false);
  if (result.ok) return;
  assert.ok(result.errors.some((e) => e.includes("focusBorder")));
});

test("校验：formatVersion 不匹配直接拒收（不同版本字段语义不可信）", () => {
  const bad = validateSkin({ ...baseSkin, formatVersion: 2, id: "!!!" });
  assert.equal(bad.ok, false);
  if (bad.ok) return;
  assert.deepEqual(bad.errors, ["formatVersion 必须是 1（收到：2）"]);
});

test("校验：坏色值/越界数值/坏 id 逐条报错", () => {
  const bad = validateSkin({
    ...baseSkin,
    id: "not ok id!",
    light: { brand: "red" },
    wallpaper: { kind: "image", imageDataUrl: "data:text/html;base64,xxx", opacity: 0, blurPx: 99, dim: 2 },
  });
  assert.equal(bad.ok, false);
  if (bad.ok) return;
  assert.ok(bad.errors.some((e) => e.includes("id")));
  assert.ok(bad.errors.some((e) => e.includes("light.brand")));
  assert.ok(bad.errors.some((e) => e.includes("imageDataUrl")));
});

test("校验：壁纸数值越界逐项报错（图源合法时，数值从左到右报首个错）", () => {
  const image = "data:image/png;base64,iVBOR";
  const opacityOnly = validateSkin({
    ...baseSkin,
    wallpaper: { kind: "image", imageDataUrl: image, opacity: 0, blurPx: 0, dim: 0 },
  });
  assert.ok(!opacityOnly.ok && opacityOnly.errors.some((e) => e.includes("wallpaper.opacity")));
  const blurOnly = validateSkin({
    ...baseSkin,
    wallpaper: { kind: "image", imageDataUrl: image, opacity: 1, blurPx: 99, dim: 0 },
  });
  assert.ok(!blurOnly.ok && blurOnly.errors.some((e) => e.includes("wallpaper.blurPx")));
  const dimOnly = validateSkin({
    ...baseSkin,
    wallpaper: { kind: "image", imageDataUrl: image, opacity: 1, blurPx: 0, dim: 2 },
  });
  assert.ok(!dimOnly.ok && dimOnly.errors.some((e) => e.includes("wallpaper.dim")));
});

test("校验：data URL 协议白名单放行图片、拒绝其他 scheme", () => {
  const ok = validateSkin({
    ...baseSkin,
    wallpaper: { kind: "image", imageDataUrl: "data:image/png;base64,iVBOR", opacity: 1, blurPx: 0, dim: 0 },
  });
  assert.equal(ok.ok, true);

  const js = validateSkin({
    ...baseSkin,
    wallpaper: { kind: "image", imageDataUrl: "data:text/javascript,alert(1)", opacity: 1, blurPx: 0, dim: 0 },
  });
  assert.equal(js.ok, false);
});

test("校验：超长 data URL 拒绝", () => {
  const result = validateSkin({
    ...baseSkin,
    wallpaper: {
      kind: "image",
      imageDataUrl: "data:image/png;base64," + "A".repeat(MAX_SKIN_IMAGE_CHARS + 1),
      opacity: 1,
      blurPx: 0,
      dim: 0,
    },
  });
  assert.equal(result.ok, false);
});

test("校验：非对象/空壳不炸", () => {
  assert.equal(validateSkin(null).ok, false);
  assert.equal(validateSkin("skin").ok, false);
  assert.equal(validateSkin([]).ok, false);
  const minimal = validateSkin({ formatVersion: 1, id: "a", name: "最小皮肤" });
  assert.equal(minimal.ok, true);
  if (minimal.ok) {
    assert.equal(minimal.skin.baseStyle, "claude");
    assert.equal(minimal.skin.wallpaper, undefined);
  }
});

test("序列化往返：serialize 后回读一致", () => {
  const skin = createSkinFromStyle("notion", "round-trip", "往返测试");
  const parsed = validateSkin(JSON.parse(serializeSkin(skin)));
  assert.equal(parsed.ok, true);
  if (parsed.ok) {
    assert.deepEqual(parsed.skin, skin);
  }
});
