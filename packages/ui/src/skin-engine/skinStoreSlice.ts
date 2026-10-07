import { validateSkin, type SkinPack, type SkinValidationError } from "./skinSchema.js";
import { applySkinToDocument } from "./skinApply.js";
import {
  readActiveSkinId,
  readSkinLibrary,
  SKIN_ACTIVE_STORAGE_KEY,
  SKIN_LIBRARY_STORAGE_KEY,
  writeActiveSkinId,
  writeSkinLibrary,
} from "./skinLibrary.js";
import type { DesignStyle } from "../themeStyles.js";

/** 皮肤导入结果：ok=入库成功；失败带逐条机器码错误；quotaFull=校验过了但存不进 localStorage */
export type SkinImportResult =
  | { ok: true; skin: SkinPack }
  | { ok: false; errors: SkinValidationError[]; quotaFull?: boolean };

type SkinSet = (partial: {
  skinLibrary?: SkinPack[];
  activeSkinId?: string | null;
  designStyle?: DesignStyle;
}) => void;
type SkinGet = () => {
  skinLibrary: SkinPack[];
  activeSkinId: string | null;
  designStyle: DesignStyle;
  setDesignStyle: (style: DesignStyle) => void;
};

/** 库内按 id 覆盖式插入（同 id 替换，其余不动）。 */
function upsertSkinInLibrary(library: SkinPack[], pack: SkinPack): SkinPack[] {
  const index = library.findIndex((entry) => entry.id === pack.id);
  if (index === -1) return [...library, pack];
  const next = [...library];
  next[index] = pack;
  return next;
}

/** 皮肤引擎的 zustand 切片：库/激活/导入/调参/删除/广播同步。 */
export function createSkinStoreSlice(set: SkinSet, get: SkinGet) {
  return {
    skinLibrary: [] as SkinPack[],
    activeSkinId: null as string | null,
    importSkin: (raw: unknown): SkinImportResult => {
      const result = validateSkin(raw);
      if (!result.ok) return result;
      // 每次写盘前先读新库：内存库只是本窗口的启动快照，别的窗口可能刚装过皮肤，
      // 拿旧快照整库回写会把人家静默抹掉。
      const library = upsertSkinInLibrary(readSkinLibrary(), result.skin);
      if (!writeSkinLibrary(library)) {
        return {
          ok: false,
          errors: [{ key: "quotaFullImport" }],
          quotaFull: true,
        };
      }
      set({ skinLibrary: library });
      return result;
    },
    upsertSkin: (skin: SkinPack) => {
      // 调参也过校验门：面板拼出来的 data URL / 数值可能非法，落库前拦下，
      // 否则重启时整条皮肤会被启动校验静默丢弃。
      const validated = validateSkin(skin);
      if (!validated.ok) return { ok: false as const, errors: validated.errors };
      const library = upsertSkinInLibrary(readSkinLibrary(), validated.skin);
      if (!writeSkinLibrary(library)) {
        return { ok: false as const, errors: [{ key: "quotaFull" }], quotaFull: true };
      }
      set({ skinLibrary: library });
      if (get().activeSkinId === validated.skin.id) {
        applySkinToDocument(validated.skin);
      }
      return { ok: true as const, skin: validated.skin };
    },
    removeSkin: (id: string) => {
      const library = readSkinLibrary().filter((pack) => pack.id !== id);
      writeSkinLibrary(library);
      if (get().activeSkinId === id) {
        writeActiveSkinId(null);
        applySkinToDocument(null);
        set({ skinLibrary: library, activeSkinId: null });
        return;
      }
      set({ skinLibrary: library });
    },
    setActiveSkinId: (id: string | null) => {
      const pack =
        id === null ? null : get().skinLibrary.find((entry) => entry.id === id) ?? null;
      writeActiveSkinId(pack?.id ?? null);
      applySkinToDocument(pack);
      // 外观下拉跟屏（换肤引擎批 2 双入口统一）：激活皮肤时预设风格同步成皮肤的
      // baseStyle，下拉显示的就是屏幕上真实生效的 ramp；停用不动（用户没说要回到
      // 哪套预设， activatePresetStyle 才负责停用+切换）。
      if (pack) get().setDesignStyle(pack.baseStyle);
      set({ activeSkinId: pack?.id ?? null });
    },
    /** 广播接收专用：对端已落库，本窗口只刷新+应用；未知 id 不回写（防覆盖对端刚写的激活标记）。 */
    syncActiveSkinFromBroadcast: (id: string | null) => {
      const library = readSkinLibrary();
      const pack = id === null ? null : library.find((entry) => entry.id === id) ?? null;
      if (pack === null && id !== null) {
        // 库里还没有（广播比库写入先到/对端是新装的）：刷新库但不动激活态
        set({ skinLibrary: library });
        return;
      }
      applySkinToDocument(pack);
      set({ skinLibrary: library, activeSkinId: pack?.id ?? null });
    },
  };
}

/**
 * 启动恢复 + 跨窗口新鲜度：从 localStorage 回读库与激活皮肤（坏条目在读取层跳过），
 * 先于首帧应用避免闪预设底色；之后监听 storage 事件——别的窗口写皮肤键会触发
 * （本窗口自己的写不触发），skinLibrary 不走广播（含大 data URL），跨窗口靠这里兜底。
 */
export function initSkinPersistence(set: SkinSet): () => void {
  const restore = () => {
    const library = readSkinLibrary();
    const activeId = readActiveSkinId();
    const pack =
      activeId === null ? null : library.find((entry) => entry.id === activeId) ?? null;
    applySkinToDocument(pack);
    set({ skinLibrary: library, activeSkinId: pack?.id ?? null });
  };
  restore();
  const onStorage = (event: StorageEvent) => {
    if (event.key !== SKIN_LIBRARY_STORAGE_KEY && event.key !== SKIN_ACTIVE_STORAGE_KEY && event.key !== null) {
      return;
    }
    restore();
  };
  window.addEventListener("storage", onStorage);
  return () => window.removeEventListener("storage", onStorage);
}
