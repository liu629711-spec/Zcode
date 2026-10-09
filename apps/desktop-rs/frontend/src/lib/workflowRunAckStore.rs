//! 1:1 翻译 `packages/ui/src/lib/workflowRunAckStore.ts`（101 行）。
//!
//! 已确认的工作流 run。已结束的运行行要一直挂着直到用户打开那个会话——没有计时器、
//! 没有 settledAt，只有这一份有界、持久化的 runId 集合：打开会话时把它当时所有已结束的
//! run 整批放进来。桌面走 localStorage（Tauri WebView 里可用），手机远控在自己的浏览器里
//! 走同一份代码、自己的存储。

use std::sync::{Mutex, OnceLock};

use leptos::prelude::*;

const WORKFLOW_RUN_ACK_STORAGE_KEY: &str = "zcode-workflow-run-acknowledged";
/// 集合上限；满了淘汰最早确认的。256 远大于任何会话列表里同时挂着的已结束 run 数。
const WORKFLOW_RUN_ACK_LIMIT: usize = 256;

/// 持久化后端（真源 `StorageLike`）：测试注入内存实现，生产走 localStorage。
trait StorageLike: Send + Sync {
    fn get_item(&self, key: &str) -> Option<String>;
    fn set_item(&self, key: &str, value: &str);
}

struct BrowserStorage;

impl StorageLike for BrowserStorage {
    fn get_item(&self, key: &str) -> Option<String> {
        web_sys::window()?
            .local_storage()
            .ok()
            .flatten()?
            .get_item(key)
            .ok()
            .flatten()
    }
    fn set_item(&self, key: &str, value: &str) {
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item(key, value);
        }
    }
}

/// `readStored`（真源 :24-35）：存储里坏掉 / 不是字符串数组时回退空表。
fn read_stored(storage: &dyn StorageLike) -> Vec<String> {
    let Some(raw) = storage.get_item(WORKFLOW_RUN_ACK_STORAGE_KEY) else {
        return Vec::new();
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    match parsed {
        serde_json::Value::Array(items) => items
            .into_iter()
            .filter_map(|entry| entry.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

/// 确认集合的状态（真源 `createWorkflowRunAckStore` 的闭包变量）。
struct AckState {
    /// 插入序 = 确认序；淘汰最早的（Vec 保插入序，与真源 Set 的迭代序一致）。
    acknowledged: Vec<String>,
    listeners: Vec<Box<dyn Fn() + Send>>,
    version: u64,
}

/// 已确认集合（真源 `WorkflowRunAckStore`）。线程安全：Leptos 的 Effect 与
/// 回调从不同作用域访问同一份全局。
pub struct WorkflowRunAckStore {
    state: Mutex<AckState>,
    storage: Box<dyn StorageLike>,
}

impl WorkflowRunAckStore {
    fn create(storage: Box<dyn StorageLike>) -> Self {
        // 真源 :39 —— readStored(...).slice(-LIMIT)：只留最后 LIMIT 条。
        let mut acknowledged = read_stored(storage.as_ref());
        if acknowledged.len() > WORKFLOW_RUN_ACK_LIMIT {
            acknowledged = acknowledged.split_off(acknowledged.len() - WORKFLOW_RUN_ACK_LIMIT);
        }
        WorkflowRunAckStore {
            state: Mutex::new(AckState {
                acknowledged,
                listeners: Vec::new(),
                version: 0,
            }),
            storage,
        }
    }

    /// `isAcknowledged`（真源 :51）。
    pub fn is_acknowledged(&self, run_id: &str) -> bool {
        self.state
            .lock()
            .map(|state| state.acknowledged.iter().any(|id| id == run_id))
            .unwrap_or(false)
    }

    /// `acknowledge`（真源 :52-68）：整批放入；没有变化就不通知、不落盘。
    /// 满了淘汰最早确认的。
    pub fn acknowledge(&self, run_ids: &[String]) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let mut changed = false;
        for run_id in run_ids {
            if state.acknowledged.iter().any(|id| id == run_id) {
                continue;
            }
            state.acknowledged.push(run_id.clone());
            changed = true;
        }
        if !changed {
            return;
        }
        while state.acknowledged.len() > WORKFLOW_RUN_ACK_LIMIT {
            state.acknowledged.remove(0);
        }
        state.version += 1;
        let payload = serde_json::to_string(&state.acknowledged).unwrap_or_else(|_| "[]".into());
        self.storage.set_item(WORKFLOW_RUN_ACK_STORAGE_KEY, &payload);
        for listener in &state.listeners {
            listener();
        }
    }

    /// `subscribe`（真源 :69-74）。
    pub fn subscribe(&self, listener: Box<dyn Fn() + Send>) {
        if let Ok(mut state) = self.state.lock() {
            state.listeners.push(listener);
        }
    }

    /// `getVersion`（真源 :75）：每次集合变化 +1。
    pub fn get_version(&self) -> u64 {
        self.state.lock().map(|state| state.version).unwrap_or(0)
    }
}

fn default_store() -> &'static WorkflowRunAckStore {
    static STORE: OnceLock<WorkflowRunAckStore> = OnceLock::new();
    STORE.get_or_init(|| {
        // 真源 getBrowserStorage :79-86 —— 拿不到 localStorage 时退内存（不跨重启）。
        let browser = web_sys::window().map(|_| Box::new(BrowserStorage) as Box<dyn StorageLike>);
        WorkflowRunAckStore::create(browser.unwrap_or_else(|| Box::new(MemoryStorage::default())))
    })
}

/// 测试内存后端（真源通过参数注入 storage；Rust 侧全局用 OnceLock，
/// 测试直接构造带内存后端的实例）。
#[derive(Default)]
struct MemoryStorage {
    items: Mutex<std::collections::HashMap<String, String>>,
}

impl StorageLike for MemoryStorage {
    fn get_item(&self, key: &str) -> Option<String> {
        self.items.lock().ok()?.get(key).cloned()
    }
    fn set_item(&self, key: &str, value: &str) {
        if let Ok(mut items) = self.items.lock() {
            items.insert(key.to_string(), value.to_string());
        }
    }
}

/// `getWorkflowRunAckStore`（真源 :90-93）。
pub fn get_workflow_run_ack_store() -> &'static WorkflowRunAckStore {
    default_store()
}

/// `useWorkflowRunAcknowledged`（真源 :96-101）：订阅确认集合的版本。
/// 真源用 `useSyncExternalStore`；Leptos 侧用信号承载版本，谓词是信号的派生读——
/// 集合一变行选择就重算。
pub fn use_workflow_run_acknowledged() -> ReadSignal<u64> {
    let store = get_workflow_run_ack_store();
    let (version, set_version) = signal(store.get_version());
    let set_version = StoredValue::new(set_version);
    store.subscribe(Box::new(move || {
        if let Some(set) = set_version.try_get_value() {
            set.set(get_workflow_run_ack_store().get_version());
        }
    }));
    version
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(entries: &[&str]) -> WorkflowRunAckStore {
        let storage = MemoryStorage::default();
        if !entries.is_empty() {
            let list: Vec<String> = entries.iter().map(|s| s.to_string()).collect();
            storage
                .set_item(WORKFLOW_RUN_ACK_STORAGE_KEY, &serde_json::to_string(&list).unwrap());
        }
        WorkflowRunAckStore::create(Box::new(storage))
    }

    #[test]
    fn empty_storage_reads_empty() {
        // 真源 :24-35 —— null / 非 JSON / 非字符串数组都回退空表。
        let store = store_with(&[]);
        assert!(!store.is_acknowledged("r1"));
        assert_eq!(store.get_version(), 0);
    }

    #[test]
    fn acknowledge_marks_and_persists() {
        let store = store_with(&[]);
        store.acknowledge(&["r1".to_string(), "r2".to_string()]);
        assert!(store.is_acknowledged("r1"));
        assert!(store.is_acknowledged("r2"));
        assert_eq!(store.get_version(), 1);
    }

    #[test]
    fn duplicate_acknowledge_is_a_noop() {
        // 真源 :54-58 —— 没有变化就不 +1、不通知。
        let store = store_with(&[]);
        store.acknowledge(&["r1".to_string()]);
        store.acknowledge(&["r1".to_string()]);
        assert_eq!(store.get_version(), 1);
    }

    #[test]
    fn evicts_oldest_beyond_limit() {
        // 真源 :59-64 —— 满了淘汰最早确认的。
        let store = WorkflowRunAckStore::create(Box::new(MemoryStorage::default()));
        let ids: Vec<String> = (0..WORKFLOW_RUN_ACK_LIMIT + 10)
            .map(|i| format!("r{i}"))
            .collect();
        store.acknowledge(&ids);
        assert_eq!(store.get_version(), 1);
        assert!(!store.is_acknowledged("r0"));
        assert!(!store.is_acknowledged("r9"));
        assert!(store.is_acknowledged("r10"));
        assert!(store.is_acknowledged(&format!("r{}", WORKFLOW_RUN_ACK_LIMIT + 9)));
    }

    #[test]
    fn persisted_state_survives_reopen() {
        // 真源：acknowledge 落盘，下一次建 store 从存储读回。
        let storage = std::sync::Arc::new(MemoryStorage::default());
        let writer = SharedStorage {
            inner: storage.clone(),
        };
        let store = WorkflowRunAckStore::create(Box::new(writer));
        store.acknowledge(&["r1".to_string()]);
        let reopened = WorkflowRunAckStore::create(Box::new(SharedStorage { inner: storage }));
        assert!(reopened.is_acknowledged("r1"));
        assert_eq!(reopened.get_version(), 0, "版本号不跨 store 存活（真源 :20-21 只存集合）");
        // 直接验证 readStored 的容错路径：坏 JSON / 非数组回退空表。
        let bad = MemoryStorage::default();
        bad.set_item(WORKFLOW_RUN_ACK_STORAGE_KEY, "not json");
        let store = WorkflowRunAckStore::create(Box::new(bad));
        assert!(!store.is_acknowledged("r1"));
        let not_array = MemoryStorage::default();
        not_array.set_item(WORKFLOW_RUN_ACK_STORAGE_KEY, "{\"a\":1}");
        let store = WorkflowRunAckStore::create(Box::new(not_array));
        assert!(!store.is_acknowledged("r1"));
    }

    struct SharedStorage {
        inner: std::sync::Arc<MemoryStorage>,
    }
    impl StorageLike for SharedStorage {
        fn get_item(&self, key: &str) -> Option<String> {
            self.inner.get_item(key)
        }
        fn set_item(&self, key: &str, value: &str) {
            self.inner.set_item(key, value)
        }
    }

    #[test]
    fn read_stored_keeps_only_strings() {
        let storage = MemoryStorage::default();
        storage.set_item(WORKFLOW_RUN_ACK_STORAGE_KEY, "[\"a\",1,\"b\",null]");
        let store = WorkflowRunAckStore::create(Box::new(storage));
        assert!(store.is_acknowledged("a"));
        assert!(store.is_acknowledged("b"));
        assert_eq!(store.get_version(), 0);
    }

    #[test]
    fn subscribers_are_notified_on_change() {
        let store = WorkflowRunAckStore::create(Box::new(MemoryStorage::default()));
        let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let hits_for_listener = hits.clone();
        store.subscribe(Box::new(move || {
            hits_for_listener.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }));
        store.acknowledge(&["r1".to_string()]);
        // 重复确认不通知。
        store.acknowledge(&["r1".to_string()]);
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
