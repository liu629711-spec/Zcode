import { validateSkin, type SkinPack } from "./skinSchema.js";

/**
 * 皮肤分享链接——纯配色皮肤编码进 URL hash（#skin=...），点开即装，不需要服务器。
 * 含壁纸 data URL 的皮肤不进链接（URL 会爆长）：返回 null，调用方引导走文件分享。
 */

export const SKIN_SHARE_HASH_PREFIX = "#skin=";

/** UTF-8 安全的 base64url（btoa 只认 Latin1，先过 TextEncoder） */
function toBase64Url(json: string): string {
  const bytes = new TextEncoder().encode(json);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(value: string): string {
  const base64 = value.replace(/-/g, "+").replace(/_/g, "/");
  const binary = atob(base64 + "=".repeat((4 - (base64.length % 4)) % 4));
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

/** 纯配色皮肤 → hash 参数值；含壁纸/含 http 图片源 → null（引导走文件） */
export function encodeSkinForShare(skin: SkinPack): string | null {
  if (skin.wallpaper) return null;
  const { wallpaper: _omitted, ...pureColor } = skin;
  return toBase64Url(JSON.stringify(pureColor));
}

export function buildSkinShareUrl(skin: SkinPack): string | null {
  const encoded = encodeSkinForShare(skin);
  if (encoded === null) return null;
  if (typeof location === "undefined") return null;
  return `${location.origin}${location.pathname}${SKIN_SHARE_HASH_PREFIX}${encoded}`;
}

export type SkinShareDecodeResult =
  | { ok: true; skin: SkinPack }
  | { ok: false; errors: string[] };

/** hash 参数值 → 皮肤包（过与文件导入同一条校验门） */
export function decodeSkinFromShare(value: string): SkinShareDecodeResult {
  try {
    return validateSkin(JSON.parse(fromBase64Url(value)));
  } catch {
    return { ok: false, errors: ["分享链接已损坏或不完整，无法解析"] };
  }
}

/** 从当前页面地址里取分享皮肤（有则消费并清掉 hash，避免刷新重复弹装）。 */
export function consumeSkinShareFromLocation(): SkinShareDecodeResult | null {
  if (typeof location === "undefined") return null;
  const hash = location.hash;
  if (!hash.startsWith(SKIN_SHARE_HASH_PREFIX)) return null;
  const value = hash.slice(SKIN_SHARE_HASH_PREFIX.length);
  history.replaceState(null, "", `${location.pathname}${location.search}`);
  return decodeSkinFromShare(value);
}
