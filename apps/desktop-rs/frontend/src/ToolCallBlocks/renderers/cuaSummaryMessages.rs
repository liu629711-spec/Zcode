//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/cuaSummaryMessages.ts`
//! （27 行）。
//!
//! CUA 工具名 → 摘要文案 id 的映射（26 项）。

/// `CUA_TOOL_SUMMARY_IDS`（真源 :5-31，25 项：工具名 → i18n id）。
pub const CUA_TOOL_SUMMARY_IDS: [(&str, &str); 25] = [
    ("request_access", "chat.toolCall.cua.requestAccess"),
    ("list_apps", "chat.toolCall.cua.listApps"),
    ("list_windows", "chat.toolCall.cua.listWindows"),
    ("get_app_state", "chat.toolCall.cua.getAppState"),
    ("screenshot", "chat.toolCall.cua.screenshot"),
    ("zoom", "chat.toolCall.cua.zoom"),
    ("open_application", "chat.toolCall.cua.openApplication"),
    ("left_click", "chat.toolCall.cua.leftClick"),
    ("double_click", "chat.toolCall.cua.doubleClick"),
    ("triple_click", "chat.toolCall.cua.tripleClick"),
    ("right_click", "chat.toolCall.cua.rightClick"),
    ("middle_click", "chat.toolCall.cua.middleClick"),
    ("scroll", "chat.toolCall.cua.scroll"),
    ("left_click_drag", "chat.toolCall.cua.drag"),
    ("mouse_move", "chat.toolCall.cua.mouseMove"),
    ("type", "chat.toolCall.cua.type"),
    ("set_value", "chat.toolCall.cua.setValue"),
    ("select_text", "chat.toolCall.cua.selectText"),
    ("key", "chat.toolCall.cua.key"),
    ("hold_key", "chat.toolCall.cua.holdKey"),
    ("perform_action", "chat.toolCall.cua.performAction"),
    ("wait", "chat.toolCall.cua.wait"),
    ("read_clipboard", "chat.toolCall.cua.readClipboard"),
    ("write_clipboard", "chat.toolCall.cua.writeClipboard"),
    ("stop_computer_control", "chat.toolCall.cua.stop"),
];

/// 工具名 → 摘要文案 id（真源 `CUA_TOOL_SUMMARY_IDS[toolName]`）。
pub fn cua_tool_summary_id(tool_name: &str) -> Option<&'static str> {
    CUA_TOOL_SUMMARY_IDS
        .iter()
        .find(|(name, _)| *name == tool_name)
        .map(|(_, id)| *id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_known_tool_names() {
        assert_eq!(
            cua_tool_summary_id("screenshot"),
            Some("chat.toolCall.cua.screenshot")
        );
        assert_eq!(
            cua_tool_summary_id("left_click_drag"),
            Some("chat.toolCall.cua.drag")
        );
        assert_eq!(cua_tool_summary_id("Bash"), None);
    }
}
