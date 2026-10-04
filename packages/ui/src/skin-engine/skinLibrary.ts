import { validateSkin, type SkinPack } from "./skinSchema.js";

/**
 * 皮肤库持久化——localStorage 两键：皮肤库数组 + 激活 id。
 * 库里含内嵌壁纸 data URL，可能很大：写入失败（配额满）必须如实上报，
 * 不能像普通偏好那样静默吞掉——用户以为装好了其实没存上。
 */

export const SKIN_LIBRARY_STORAGE_KEY = "zcode-skin-library";
export const SKIN_ACTIVE_STORAGE_KEY = "zcode-skin-active-id";

function getLocalStorage(): Storage | null {
  try {
    if (typeof localStorage === "undefined") return null;
    return localStorage;
  } catch {
    return null;
  }
}

/** 回读皮肤库：逐条过校验门，坏条目跳过（不因一条坏数据毁掉整库）。 */
export function readSkinLibrary(): SkinPack[] {
  const raw = getLocalStorage()?.getItem(SKIN_LIBRARY_STORAGE_KEY);
  if (!raw) return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const packs: SkinPack[] = [];
    for (const entry of parsed) {
      const result = validateSkin(entry);
      if (result.ok) packs.push(result.skin);
    }
    return packs;
  } catch {
    return [];
  }
}

export function readActiveSkinId(): string | null {
  return getLocalStorage()?.getItem(SKIN_ACTIVE_STORAGE_KEY) || null;
}

/** 写皮肤库；false = 配额满/存储不可写（调用方须提示用户）。 */
export function writeSkinLibrary(packs: SkinPack[]): boolean {
  try {
    getLocalStorage()?.setItem(SKIN_LIBRARY_STORAGE_KEY, JSON.stringify(packs));
    return true;
  } catch {
    return false;
  }
}

export function writeActiveSkinId(id: string | null): boolean {
  try {
    const storage = getLocalStorage();
    if (!storage) return false;
    if (id === null) storage.removeItem(SKIN_ACTIVE_STORAGE_KEY);
    else storage.setItem(SKIN_ACTIVE_STORAGE_KEY, id);
    return true;
  } catch {
    return false;
  }
}
