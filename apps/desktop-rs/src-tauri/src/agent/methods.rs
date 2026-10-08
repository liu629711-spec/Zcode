//! 协议方法名常量（直译 `packages/shared/src/zcode-protocol/index.ts` 的
//! zcodeProtocolMethods，3645-3758 行）。方法名是 wire 事实，不得改动。

pub const RUNTIME_CAPABILITIES: &str = "runtime/capabilities";
pub const COMPUTER_USE_OPERATION_EVENT: &str = "computer-use/operation-event";
pub const SESSION_CREATE: &str = "session/create";
pub const SESSION_RESUME: &str = "session/resume";
pub const SESSION_LIST: &str = "session/list";
pub const SESSION_SUBAGENTS: &str = "session/subagents";
pub const SESSION_REQUEST_RUNTIME_PREFERENCES: &str = "session/requestRuntimePreferences";
pub const SESSION_READ: &str = "session/read";
pub const SESSION_MESSAGES: &str = "session/messages";
pub const SESSION_EVENTS: &str = "session/events";
pub const SESSION_DEBUG: &str = "session/debug";
pub const SESSION_SUBSCRIBE: &str = "session/subscribe";
/// deprecated（部分）：send 主路径已收敛 v4 sendText，仅附件回退分支消费。
pub const SESSION_SEND: &str = "session/send";
/// deprecated：stop 已收敛 v4 stop 命令，wire case 留兼容。
pub const SESSION_STOP: &str = "session/stop";
/// deprecated：已收敛 v4 cancelBackgroundWork 命令。
pub const SESSION_CANCEL_BACKGROUND_TASK: &str = "session/cancelBackgroundTask";
/// deprecated：已收敛 v4 forkAssistant。
pub const SESSION_FORK: &str = "session/fork";
pub const SESSION_COMPACT: &str = "session/compact";
pub const SESSION_GOAL: &str = "session/goal";
pub const SESSION_ARCHIVE: &str = "session/archive";
pub const SESSION_PURGE_CONTENT: &str = "session/purgeContent";
pub const SESSION_CLOSE: &str = "session/close";
pub const SESSION_SET_MODEL: &str = "session/setModel";
pub const SESSION_SET_THOUGHT_LEVEL: &str = "session/setThoughtLevel";
pub const SESSION_SET_MODE: &str = "session/setMode";
pub const WORKSPACE_READ_PRESENTATION: &str = "workspace/readPresentation";
pub const WORKSPACE_HOOK_TRUST_GRANT: &str = "workspace/hooks/trustGrant";
pub const PROVIDER_UPDATE_ACCOUNT_CONFIG: &str = "provider/updateAccountConfig";
pub const WORKSPACE_UPDATE_INTERACTION_PREFERENCES: &str = "workspace/updateInteractionPreferences";
pub const WORKSPACE_UPDATE_MODEL_IO_PREFERENCES: &str = "workspace/updateModelIoPreferences";
pub const WORKSPACE_UPDATE_OFF_PEAK_TOOL_POLICY: &str = "workspace/updateOffPeakToolPolicy";
pub const WORKSPACE_UPDATE_DYNAMIC_WORKFLOW_POLICY: &str =
    "workspace/updateDynamicWorkflowPolicy";
pub const WORKSPACE_GENERATE_TEXT: &str = "workspace/generateText";
pub const WORKSPACE_CANCEL_GENERATE_TEXT: &str = "workspace/cancelGenerateText";
pub const PROVIDER_TEST_MODEL_CONNECTIVITY: &str = "provider/testModelConnectivity";
pub const MCP_LIST: &str = "mcp/list";
pub const PLUGINS_LIST: &str = "plugins/list";
pub const PLUGINS_REFERENCE_CATALOG: &str = "plugins/referenceCatalog";
pub const PLUGINS_REFERENCE_CATALOG_WITH_CATEGORY: &str = "plugins/referenceCatalogWithCategory";
pub const SKILLS_REFERENCE_CATALOG: &str = "skills/referenceCatalog";
pub const WORKFLOWS_LIST: &str = "workflows/list";
pub const WORKFLOWS_GET: &str = "workflows/get";
pub const WORKFLOWS_UPDATE_META: &str = "workflows/updateMeta";
pub const WORKFLOWS_DELETE: &str = "workflows/delete";
pub const WORKFLOWS_RUNS: &str = "workflows/runs";
pub const WORKFLOWS_MOVE: &str = "workflows/move";
pub const PLUGINS_RESOLVE_SUGGESTED_REFERENCE: &str = "plugins/resolveSuggestedReference";
pub const PLUGINS_SET_ENABLED: &str = "plugins/setEnabled";
pub const PLUGINS_OVERVIEW: &str = "plugins/overview";
pub const PLUGINS_MARKETPLACE_ADD: &str = "plugins/marketplace/add";
pub const PLUGINS_MARKETPLACE_REMOVE: &str = "plugins/marketplace/remove";
pub const PLUGINS_MARKETPLACE_UPDATE: &str = "plugins/marketplace/update";
pub const PLUGINS_INSTALL: &str = "plugins/install";
pub const PLUGINS_CANCEL_OPERATION: &str = "plugins/cancelOperation";
pub const PLUGINS_UNINSTALL: &str = "plugins/uninstall";
pub const PLUGINS_UPDATE: &str = "plugins/update";
pub const PLUGINS_RESTORE_BUILTIN: &str = "plugins/restoreBuiltin";
pub const PLUGINS_CONFIGURE: &str = "plugins/configure";
pub const PLUGINS_RESET_CONFIG: &str = "plugins/resetConfig";
pub const PLUGINS_VALIDATE: &str = "plugins/validate";
pub const PLUGINS_DESCRIBE: &str = "plugins/describe";
pub const AUTOMATION_CREATE: &str = "automation/create";
pub const AUTOMATION_UPDATE: &str = "automation/update";
pub const AUTOMATION_CHECK_TASK_BINDING: &str = "automation/checkTaskBinding";
pub const AUTOMATION_LIST: &str = "automation/list";
pub const AUTOMATION_DELETE: &str = "automation/delete";
/// AgentDispatch 复用前的工位退役反查（CLI→Host 单向问答）。
pub const AGENT_DISPATCH_CHECK_TASK_RETIRED: &str = "agentDispatch/checkTaskRetired";
pub const OFF_PEAK_CREATE: &str = "offPeak/create";
pub const OFF_PEAK_LIST: &str = "offPeak/list";
/// deprecated：host 已改走 v4/conversation/usage。
pub const USAGE_STATS: &str = "usage/stats";
/// deprecated：host 已改走 v4/conversation/usage。
pub const SESSION_USAGE: &str = "session/usage";
pub const PROCESS_CHILD_PROCESSES: &str = "process/childProcesses";
pub const INTERACTION_REQUEST_PERMISSION: &str = "interaction/requestPermission";
pub const INTERACTION_REQUEST_USER_INPUT: &str = "interaction/requestUserInput";
pub const INTERACTION_REQUEST_PROVIDER_RUNTIME_HEADERS: &str =
    "interaction/requestProviderRuntimeHeaders";
pub const INTERACTION_REQUEST_OFFICIAL_MCP_AUTH_HEADERS: &str =
    "interaction/requestOfficialMcpAuthHeaders";
pub const INTERACTION_BROWSER_LIST: &str = "interaction/browserList";
pub const INTERACTION_BROWSER_EXECUTE: &str = "interaction/browserExecute";

/// V4 命令通道（zcode-protocol-v4/transport.ts V4_METHODS.command）。
/// 上行就是普通 JSON-RPC request，params = CommandEnvelope。
pub const V4_COMMAND: &str = "v4/command";
/// v4 会话行分页拉取（ConversationRowView 数据源；rows 升序 + hasMore）。
pub const V4_CONVERSATION_ROWS_RANGE: &str = "v4/conversation/rowsRange";

/// 通知方法名（zcodeProtocolNotifications）。
pub mod notifications {
    pub const STORAGE_STARTUP: &str = "startup/storageState";
    pub const PROVIDER_RUNTIME_HEADERS_CANCELLED: &str =
        "interaction/providerRuntimeHeadersCancelled";
    pub const MCP_TELEMETRY: &str = "process/mcpTelemetry";
    pub const MCP_RESOURCE_SAMPLES: &str = "process/mcpResourceSamples";
    pub const TOOL_EXEC_RESOURCE: &str = "process/toolExecResource";
    pub const PLUGIN_OPERATION_PROGRESS: &str = "plugins/operationProgress";
    pub const PROCESS_RESOURCE_SAMPLE: &str = "process/resourceSample";
}
