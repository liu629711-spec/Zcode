//! 1:1 翻译 `packages/ui/src/workspace-file-tree/useWorkspaceFileTreeWatchers.ts`（107 行）。
//!
//! 目录监听的**登记生命周期**：watched 集合每次变化都对齐注册表——
//! 新目录发起 watch、消失的目录释放、pending 去重、卸载时整表释放。
//!
//! 与真源的偏离（Rust 侧）：
//! - 真源注册表是 React ref + useEffect；Rust 侧拆成「纯登记表（可单测）+
//!   Effect 薄接线」两层，语义一一对应（含真源 :70-75 的迟到注册立即释放）。
//! - 服务面从 `IFileWatcherService` 换成主进程命令 `fs_watch` / `fs_unwatch`
//!   与 `fs-watcher-change` 事件（本批在 src-tauri/lib.rs 落地）。

use std::collections::{HashMap, HashSet};

/// 登记表产出的命令（真源直接调服务；Rust 侧由接线层执行 invoke）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherCommand {
    Watch { dir_path: String },
    Unwatch { watcher_id: String, dir_path: String },
}

/// watcher 注册表（真源 :15-21 的五个 ref 收拢成一张表 + 代数）。
#[derive(Debug, Default)]
pub struct WatcherRegistry {
    generation: u64,
    /// dirPath → watcher id（watch 成功后登记）。
    registered: HashMap<String, String>,
    /// 已发起、尚未拿到 id 的目录（真源 pendingWatcherDirectoryPathsRef）。
    pending: HashSet<String>,
}

impl WatcherRegistry {
    /// 真源 :49-54 —— 移除不再被监视的目录。
    /// 真源 :56-64 —— 新目录入 pending 并发起 watch（已在册/在途的跳过）。
    pub fn sync(&mut self, watched: &HashSet<String>) -> Vec<WatcherCommand> {
        let mut commands = Vec::new();
        let stale: Vec<(String, String)> = self
            .registered
            .iter()
            .filter(|(path, _)| !watched.contains(*path))
            .map(|(path, id)| (path.clone(), id.clone()))
            .collect();
        for (path, id) in stale {
            self.registered.remove(&path);
            commands.push(WatcherCommand::Unwatch {
                watcher_id: id,
                dir_path: path,
            });
        }
        for path in watched {
            if self.registered.contains_key(path) || self.pending.contains(path) {
                continue;
            }
            self.pending.insert(path.clone());
            commands.push(WatcherCommand::Watch {
                dir_path: path.clone(),
            });
        }
        commands
    }

    /// 真源 :66-86 —— watch 成功：出 pending、入 registered；
    /// 迟到（代数已变或目录已不在集合）则立即释放。
    pub fn on_watch_ok(
        &mut self,
        dir_path: &str,
        watcher_id: String,
        watched: &HashSet<String>,
    ) -> Option<WatcherCommand> {
        self.pending.remove(dir_path);
        if !watched.contains(dir_path) {
            return Some(WatcherCommand::Unwatch {
                watcher_id,
                dir_path: dir_path.to_string(),
            });
        }
        self.registered.insert(dir_path.to_string(), watcher_id);
        None
    }

    /// 真源 :87-93 —— watch 失败：出 pending 即可（下轮 sync 会重试）。
    pub fn on_watch_failed(&mut self, dir_path: &str) {
        self.pending.remove(dir_path);
    }

    /// 真源 :37-44 —— 服务替换 / 真源 :99-116 —— 卸载：代数 +1、整表释放。
    pub fn release_all(&mut self) -> Vec<WatcherCommand> {
        self.generation += 1;
        self.pending.clear();
        let commands: Vec<WatcherCommand> = self
            .registered
            .iter()
            .map(|(path, id)| WatcherCommand::Unwatch {
                watcher_id: id.clone(),
                dir_path: path.clone(),
            })
            .collect();
        self.registered.clear();
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sync_watches_new_and_unwatches_stale() {
        // 真源 :49-64 —— 集合差分（HashSet 迭代无序，按集合断言）。
        let mut registry = WatcherRegistry::default();
        let commands = registry.sync(&set(&["a", "b"]));
        assert_eq!(commands.len(), 2);
        assert!(commands.iter().any(|c| matches!(
            c,
            WatcherCommand::Watch { dir_path } if dir_path == "a"
        )));
        assert!(commands.iter().any(|c| matches!(
            c,
            WatcherCommand::Watch { dir_path } if dir_path == "b"
        )));
        // 已在途的目录不重复发起（pending 去重）。
        let commands = registry.sync(&set(&["a", "b"]));
        assert!(commands.is_empty());

        // watch 成功后进 registered。
        registry.on_watch_ok("a", "id-a".into(), &set(&["a", "b"]));
        // 集合收缩 → 释放消失的目录。
        let commands = registry.sync(&set(&["b"]));
        assert_eq!(
            commands,
            vec![WatcherCommand::Unwatch {
                watcher_id: "id-a".into(),
                dir_path: "a".into(),
            }]
        );
    }

    #[test]
    fn late_registration_is_released_immediately() {
        // 真源 :70-75 —— watch 返回时目录已不被监视 → 立即 unwatch。
        let mut registry = WatcherRegistry::default();
        let _ = registry.sync(&set(&["a"]));
        let command = registry.on_watch_ok("a", "id-a".into(), &HashSet::new());
        assert_eq!(
            command,
            Some(WatcherCommand::Unwatch {
                watcher_id: "id-a".into(),
                dir_path: "a".into(),
            })
        );
        // 迟到释放不占用 registered。
        assert!(registry.registered.is_empty());
    }

    #[test]
    fn release_all_bumps_generation_and_flushes() {
        // 真源 :99-116 —— 卸载语义：全部释放，pending 清空。
        let mut registry = WatcherRegistry::default();
        let _ = registry.sync(&set(&["a"]));
        registry.on_watch_ok("a", "id-a".into(), &set(&["a"]));
        let commands = registry.release_all();
        assert_eq!(registry.generation, 1);
        assert!(commands.len() == 1);
        assert!(registry.registered.is_empty());
        assert!(registry.pending.is_empty());
    }
}
