//! Agent 进程命令解析。
//!
//! 对应 `packages/services/src/zcode-agent/zcodeAgentProcessManager.ts`
//! 的 resolveDefaultZCodeAgentCommand（388-464 行）解析顺序。
//! 第一版覆盖开发链路（monorepo dist / 源码 tsx）与 env 显式覆盖；
//! 桌面打包态（Electron Node runtime）与 SSH 远端形态后续按需补齐。

use std::path::{Path, PathBuf};

/// 一条完整的 Agent 启动命令。
#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// 额外环境变量（如 ELECTRON_RUN_AS_NODE=1）。
    pub env: Vec<(String, String)>,
}

/// 从仓库根向上查找相对路径（对应 TS 侧 findUpward）。
fn find_upward(start: &Path, relative: &str) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(relative);
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// 解析 Agent 启动命令。
///
/// 顺序（与 TS 侧一致）：
/// 1. `ZCODE_AGENT_SERVER_COMMAND` env 显式覆盖（args 可用 `ZCODE_AGENT_SERVER_ARGS_JSON`）
/// 2. monorepo dev：`apps/zcode-cli/packages/cli/dist/zcode.cjs` + `app-server --stdio`
/// 3. 源码 dev：tsx 跑 `apps/zcode-cli/packages/cli/src/main.ts`
pub fn resolve_agent_command(workspace_path: &Path) -> Option<AgentCommand> {
    if let Ok(raw) = std::env::var("ZCODE_AGENT_SERVER_COMMAND") {
        let command = raw.trim().to_string();
        if !command.is_empty() {
            let args = std::env::var("ZCODE_AGENT_SERVER_ARGS_JSON")
                .ok()
                .and_then(|raw| {
                    serde_json::from_str::<Vec<String>>(&raw)
                        .ok()
                        .or_else(|| serde_json::from_str::<Vec<String>>(&raw).ok())
                })
                .unwrap_or_else(|| vec!["app-server".into(), "--stdio".into()]);
            let cwd = std::env::var("ZCODE_AGENT_SERVER_CWD")
                .ok()
                .filter(|c| !c.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| workspace_path.to_path_buf());
            return Some(AgentCommand {
                program: command,
                args,
                cwd,
                env: Vec::new(),
            });
        }
    }

    // 从当前 exe 位置向上找 monorepo 根。开发态 exe 位于
    // <repo>/apps/desktop-rs/target/debug/，向上 4 级即仓库根。
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if let Some(entrypoint) = find_upward(&exe_dir, "apps/zcode-cli/packages/cli/dist/zcode.cjs") {
        return Some(AgentCommand {
            program: "node".into(),
            args: vec![
                entrypoint.to_string_lossy().into_owned(),
                "app-server".into(),
                "--stdio".into(),
            ],
            cwd: workspace_path.to_path_buf(),
            env: Vec::new(),
        });
    }

    // 源码回退：tsx + src/main.ts。
    let source = find_upward(&exe_dir, "apps/zcode-cli/packages/cli/src/main.ts")?;
    // tsx 的 bin 在 <repo>/node_modules/.bin/tsx（Windows 下为 tsx.cmd，由 spawn shell 解析）。
    find_upward(&exe_dir, "node_modules/.bin/tsx").map(|tsx| AgentCommand {
        program: tsx.to_string_lossy().into_owned(),
        args: vec![
            source.to_string_lossy().into_owned(),
            "app-server".into(),
            "--stdio".into(),
        ],
        cwd: workspace_path.to_path_buf(),
        env: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_upward_finds_repo_file() {
        // 本测试随仓库运行，向上一定能找到 zcode.cjs（若已构建）或 src/main.ts。
        let start = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let found_src = find_upward(&start, "apps/zcode-cli/packages/cli/src/main.ts");
        assert!(found_src.is_some(), "应能向上找到 zcode-cli 源码入口");
    }
}
