/**
 * 视频壁纸素材仓（2026-10-07 补第 2 期欠账）：视频本体存 IndexedDB，皮肤包只记
 * 引用 id（kind=video 的 videoAssetId）。
 *
 * 为什么不上传成 data URL / 本地路径：
 * - data URL 进 localStorage：几十 MB 的视频会直接压爆 5MB 配额（图片上限 4MB 已是
 *   该配额的极限）；
 * - 本地绝对路径：绑死机器、手机远控端读不到，还要拖着主进程媒体授权协议走
 *   （30 分钟时效，壁纸要挂一整天，错配）。
 * IndexedDB 没有 5MB 那种小配额、原生存 Blob，且重启/换会话都不失效；代价是换机器
 * 或清过浏览器数据后引用查不到——壁纸层静默回退无壁纸，皮肤其余部分照常。
 *
 * 清场纪律：删皮肤/换壁纸时按引用回收（见 releaseSkinVideoAsset 调用点），孤儿
 * 素材由 pruneSkinVideoAssets 在每次写入后顺带清掉（最多几十 MB，不做精细 GC）。
 */

const VIDEO_ASSET_DB_NAME = "zcode-skin-video-assets";
const VIDEO_ASSET_DB_VERSION = 1;
const VIDEO_ASSET_STORE = "videos";

/** 单条视频上限（防误选超大文件拖垮内存；4K 短片/常规桌面视频都在此以内）。 */
export const MAX_SKIN_VIDEO_BYTES = 200 * 1024 * 1024;

function openVideoAssetDb(): Promise<IDBDatabase | null> {
  if (typeof indexedDB === "undefined") return Promise.resolve(null);
  return new Promise((resolvePromise) => {
    let request: IDBOpenDBRequest;
    try {
      request = indexedDB.open(VIDEO_ASSET_DB_NAME, VIDEO_ASSET_DB_VERSION);
    } catch {
      resolvePromise(null);
      return;
    }
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(VIDEO_ASSET_STORE)) {
        db.createObjectStore(VIDEO_ASSET_STORE);
      }
    };
    request.onsuccess = () => resolvePromise(request.result);
    request.onerror = () => resolvePromise(null);
  });
}

function runVideoAssetTx<T>(
  mode: IDBTransactionMode,
  run: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T | null> {
  return openVideoAssetDb().then(
    (db) =>
      new Promise<T | null>((resolvePromise) => {
        if (!db) {
          resolvePromise(null);
          return;
        }
        try {
          const tx = db.transaction(VIDEO_ASSET_STORE, mode);
          const request = run(tx.objectStore(VIDEO_ASSET_STORE));
          request.onsuccess = () => resolvePromise(request.result);
          request.onerror = () => resolvePromise(null);
          tx.oncomplete = () => db.close();
          tx.onerror = () => {
            db.close();
            resolvePromise(null);
          };
        } catch {
          db.close();
          resolvePromise(null);
        }
      }),
  );
}

/** 新素材 id：皮肤内去重用的稳定新键（时间戳+随机尾巴，够用且可读）。 */
export function createSkinVideoAssetId(): string {
  return `vid-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

/** 存一段视频 → 返回可写进皮肤包的引用 id；失败（超限/浏览器不支持）返回 null。 */
export async function putSkinVideoAsset(assetId: string, blob: Blob): Promise<boolean> {
  if (blob.size > MAX_SKIN_VIDEO_BYTES) return false;
  const result = await runVideoAssetTx("readwrite", (store) => store.put(blob, assetId));
  return result !== null || (await readSkinVideoAsset(assetId)) !== null;
}

/** 取视频本体（壁纸层渲染用）；查不到返回 null（壁纸静默回退）。 */
export function readSkinVideoAsset(assetId: string): Promise<Blob | null> {
  return runVideoAssetTx<Blob>("readonly", (store) => store.get(assetId));
}

/** 回收一条素材（删皮肤/换壁纸时调用）。 */
export async function deleteSkinVideoAsset(assetId: string): Promise<void> {
  await runVideoAssetTx("readwrite", (store) => store.delete(assetId));
}

/**
 * 孤儿清场：库与皮肤包引用对账，删掉没人引用的素材。每次写入后顺带跑一次
 * （引用集合小、素材条数更小，不做增量索引）。
 */
export async function pruneSkinVideoAssets(referencedIds: readonly string[]): Promise<void> {
  const keys = await runVideoAssetTx<IDBValidKey[]>("readonly", (store) => store.getAllKeys());
  if (!keys) return;
  const referenced = new Set(referencedIds);
  await Promise.all(
    keys
      .map((key) => String(key))
      .filter((key) => !referenced.has(key))
      .map((key) => deleteSkinVideoAsset(key)),
  );
}
