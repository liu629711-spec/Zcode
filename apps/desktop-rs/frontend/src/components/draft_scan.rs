//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/draft-scan.ts`（77 行）。
//!
//! 流式草稿：模型还在写脚本、分析器还没跑，卡上就先把**阶段线**画出来——
//! 只有站与淡墨轨道段，没有药丸、没有弧、没有状态；子代理只计数（表头的 `N phases · M agents`），
//! 画面留给分析器。
//!
//! ★真源 :5-13 注释——这是一个**正则级扫描器**，不是解析器——它只认 `phase("…")` 与
//! `agent("…")`，认错了也无妨：display 一到整个模型就被分析器的替换掉（不变式 10）。
//!
//! 同名的第二个 `phase("implement")` 是回到那一站（分析器会把它折成回边），不是新站——草稿
//! **只加不减**，站数与分析器的一致，交接时不会有站消失。最后一个未闭合的 `phase("ver` 给
//! 最后一站 `typing`。

use crate::components::timeline_model::{
    DraftAgents, TimelineInk, TimelineRail, TimelineStation, WorkflowTimelineModel,
};
use crate::components::workflow_graph::phase_name::PhaseNaming;

/// `WorkflowDraftPhase`（真源 :14-17）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDraftPhase {
    pub name: String,
    /// 最后一个未闭合的标记（流式下正在写）。
    pub typing: bool,
}

/// `WorkflowDraft`（真源 :19-23）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowDraft {
    pub phases: Vec<WorkflowDraftPhase>,
    /// 按出现顺序去重的子代理名（只计数，不画）。
    pub agents: Vec<String>,
}

/// 一次匹配（真源 :25 正则的捕获组）。
struct Token {
    /// `"phase"` 或 `"agent"`。
    kind: String,
    /// 引号字符（`"` / `'` / 反引号）；`None` = 没有引号（未闭合或无引号）。
    quote: Option<char>,
    /// 引号里的内容（可能为空）。
    body: String,
    /// 闭合引号是否在场。
    closed: bool,
}

/// `TOKEN`（真源 :25）的等价实现：`\b(phase|agent)\(\s*(?:(["'\`])([^"'\`\n]*)(\2)?)?`
///
/// 手写扫描器而不是拉正则依赖：这个模式只需要「词边界 + 括号 + 可选引号 + 同行内容」，
/// 手写换来的是**不引新依赖**（前端 crate 现在只有 regex-lite）以及流式场景下的可控性
/// （`${` 插值按真源只取头部）。
fn scan_tokens(script: &str) -> Vec<Token> {
    let bytes: Vec<char> = script.chars().collect();
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        // `\b(phase|agent)\(`：词首（前一字符不是词字符）+ 关键字 + 紧跟的左括号。
        if !is_word_start(bytes.get(i.wrapping_sub(1)).copied(), c) {
            i += 1;
            continue;
        }
        let kind = match_keyword(&bytes, i);
        let Some(kind) = kind else {
            i += 1;
            continue;
        };
        let mut j = i + kind.len();
        if bytes.get(j) != Some(&'(') {
            i += 1;
            continue;
        }
        j += 1;
        // `\s*`
        while matches!(bytes.get(j), Some(ch) if ch.is_whitespace()) {
            j += 1;
        }
        // `(?:(["'\`])([^"'\`\n]*)(\2)?)?` —— 整组可省；引号组也可省。
        let quote = match bytes.get(j) {
            Some(&ch @ ('"' | '\'' | '`')) => Some(ch),
            _ => None,
        };
        let Some(quote) = quote else {
            // 没有引号：整组不匹配，真源解构出的 `quote` 是 `undefined` → 跳过。
            out.push(Token {
                kind,
                quote: None,
                body: String::new(),
                closed: false,
            });
            i = j;
            continue;
        };
        j += 1;
        let mut body = String::new();
        let mut closed = false;
        while let Some(&ch) = bytes.get(j) {
            if ch == quote {
                j += 1;
                closed = true;
                break;
            }
            // `[^"'\`\n]*` —— 换行即止（真源 :25 的字符类里没有 `\n`）。
            if ch == '\n' || ch == '"' || ch == '\'' || ch == '`' {
                break;
            }
            body.push(ch);
            j += 1;
        }
        out.push(Token {
            kind,
            quote: Some(quote),
            body,
            closed,
        });
        i = j;
    }
    out
}

/// `\b` 的左半：前一字符不是词字符（`\w` = 字母 / 数字 / 下划线）。
fn is_word_start(prev: Option<char>, c: char) -> bool {
    if !c.is_alphanumeric() && c != '_' {
        return false;
    }
    !matches!(prev, Some(p) if p.is_alphanumeric() || p == '_')
}

/// 在位置 `i` 上试一个关键字，返回它与它的长度。
fn match_keyword(chars: &[char], i: usize) -> Option<String> {
    for keyword in ["phase", "agent"] {
        let k: Vec<char> = keyword.chars().collect();
        if chars.get(i..i + k.len())? == k.as_slice() {
            return Some(keyword.to_string());
        }
    }
    None
}

/// `scanWorkflowDraft`（真源 :27-52）。
pub fn scan_workflow_draft(script: &str) -> WorkflowDraft {
    let mut phases: Vec<WorkflowDraftPhase> = Vec::new();
    let mut agents: Vec<String> = Vec::new();
    for token in scan_tokens(script) {
        if token.quote.is_none() {
            continue;
        }
        // ★真源 :33-34 —— 模板字面量里的插值只取头部：
        // `研究员${i}` 记成「研究员」——名字的形状比空着强。
        let name = token
            .body
            .split("${")
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let closed = token.closed;
        if token.kind == "phase" {
            // ★真源 :37-43 —— 已经在 typing 的站就是这个标记本身：
            // 流式下同一处会被扫到多次，定名而不是再开一站。
            if let Some(last) = phases.last_mut() {
                if last.typing {
                    last.name = name;
                    if closed {
                        last.typing = false;
                    }
                    continue;
                }
            }
            // ★真源 :44 —— 同名的第二个 `phase("implement")` 是回到那一站，
            // 不是新站（分析器会把它折成回边）。
            if closed && phases.iter().any(|p| p.name == name) {
                continue;
            }
            phases.push(WorkflowDraftPhase {
                name,
                typing: !closed,
            });
            continue;
        }
        if !closed || name.is_empty() {
            continue;
        }
        if !agents.iter().any(|a| a == &name) {
            agents.push(name);
        }
    }
    WorkflowDraft { phases, agents }
}

/// `draftTimeline`（真源 :55-76）：草稿 → 时间线模型。
///
/// 只有站与淡墨轨道段，没有药丸、没有弧、没有状态。
pub fn draft_timeline(draft: &WorkflowDraft) -> WorkflowTimelineModel {
    let stations: Vec<TimelineStation> = draft
        .phases
        .iter()
        .enumerate()
        .map(|(i, phase)| TimelineStation {
            id: format!("draft:{i}"),
            naming: PhaseNaming {
                id: format!("draft:{i}"),
                name: Some(phase.name.clone()),
            },
            pills: Vec::new(),
            status: None,
            visited: false,
            rounds: 0,
            on_loop: false,
            track: 0,
            fraction: None,
            unlisted: None,
            typing: phase.typing,
        })
        .collect();
    let rails: Vec<TimelineRail> = stations
        .iter()
        .skip(1)
        .enumerate()
        .map(|(i, _)| TimelineRail {
            from: i,
            to: i + 1,
            ink: TimelineInk::Faint,
            kind: None,
        })
        .collect();
    WorkflowTimelineModel {
        arcs: Vec::new(),
        bands: Vec::new(),
        draft: Some(DraftAgents {
            agents: draft.agents.len(),
        }),
        live: false,
        rails,
        running_index: None,
        stations,
        stalled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(draft: &WorkflowDraft) -> Vec<&str> {
        draft.phases.iter().map(|p| p.name.as_str()).collect()
    }

    // ── 扫描 ──

    #[test]
    fn scans_phases_and_agents() {
        let script = r#"
            phase("prepare")
            agent("研究员")
            phase("implement")
            agent("审查员")
        "#;
        let draft = scan_workflow_draft(script);
        assert_eq!(names(&draft), vec!["prepare", "implement"]);
        assert_eq!(draft.agents, vec!["研究员", "审查员"]);
    }

    #[test]
    fn word_boundary_keeps_similar_names_out() {
        // 真源 :25 —— `\b(phase|agent)\(` 前面必须是词首。
        // `my_phase("x")` / `subagent("y")` 都不该被认出来。
        let draft = scan_workflow_draft(r#"my_phase("x") subagent("y")"#);
        assert!(draft.phases.is_empty());
        assert!(draft.agents.is_empty());
    }

    #[test]
    fn all_three_quote_styles_work() {
        // 真源 :25 —— 引号类是 `["'\`]`。
        for q in ['"', '\'', '`'] {
            let script = format!("phase({q}plan{q})");
            let draft = scan_workflow_draft(&script);
            assert_eq!(names(&draft), vec!["plan"], "引号 {q}");
        }
    }

    #[test]
    fn unquoted_call_is_skipped() {
        // 真源 :32 —— quote 为 undefined 直接 continue。
        let draft = scan_workflow_draft("phase(prepare) agent(worker)");
        assert!(draft.phases.is_empty());
        assert!(draft.agents.is_empty());
    }

    #[test]
    fn body_stops_at_newline_and_the_station_stays_typing() {
        // 真源 :25 —— 字符类 `[^"'\`\n]*` 里没有 `\n`：body 只吃到换行前，
        // 闭合组 `(\2)?` 是可选的所以匹配仍然成立 → 得到一个**没闭合**的站。
        let draft = scan_workflow_draft("phase(\"plan\nrest\")");
        assert_eq!(names(&draft), vec!["plan"], "名字只取换行前那一截");
        assert!(draft.phases[0].typing, "闭合引号没吃到 → typing");
    }

    #[test]
    fn mismatched_quote_does_not_close_the_token() {
        // `phase("plan')` —— 开引号是 `"`，闭引号是 `'`：body 停在 `'` 之前，
        // 闭合组不匹配 → typing，名字是 `plan`。
        let draft = scan_workflow_draft(r#"phase("plan')"#);
        assert_eq!(names(&draft), vec!["plan"]);
        assert!(draft.phases[0].typing);
    }

    #[test]
    fn template_interpolation_keeps_only_the_head() {
        // ★真源 :33-34 —— `研究员${i}` 记成「研究员」：名字的形状比空着强。
        let draft = scan_workflow_draft("agent(`研究员${i + 1}`)");
        assert_eq!(draft.agents, vec!["研究员"]);
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        // 真源 :25 的 `\s*` + :34 的 `.trim()`。
        let draft = scan_workflow_draft("phase(  \"  plan  \"  )");
        assert_eq!(names(&draft), vec!["plan"]);
    }

    #[test]
    fn agents_dedupe_in_first_appearance_order() {
        // 真源 :49 —— `if (!agents.includes(name)) agents.push(name)`。
        let draft = scan_workflow_draft(
            r#"agent("乙") agent("甲") agent("乙") agent("甲") agent("丙")"#,
        );
        assert_eq!(draft.agents, vec!["乙", "甲", "丙"]);
    }

    // ── 流式：只加不减 ──

    #[test]
    fn repeated_phase_name_is_a_reentry_not_a_new_station() {
        // ★真源 :11-12 / :44 —— 同名的第二个 `phase("implement")` 是回到那一站，
        // 不是新站。草稿只加不减，站数与分析器的一致，交接时不会有站消失。
        let draft = scan_workflow_draft(
            r#"phase("plan") phase("implement") phase("plan") agent("x")"#,
        );
        assert_eq!(names(&draft), vec!["plan", "implement"], "第三次是再入");
    }

    #[test]
    fn unclosed_phase_marks_the_last_station_typing() {
        // ★真源 :13 —— 最后一个未闭合的 `phase("ver` 给最后一站 typing。
        let draft = scan_workflow_draft(r#"phase("plan") phase("ver"#);
        assert_eq!(names(&draft), vec!["plan", "ver"]);
        assert!(!draft.phases[0].typing, "已闭合的站不是 typing");
        assert!(draft.phases[1].typing);
    }

    #[test]
    fn typing_station_is_renamed_not_duplicated() {
        // ★真源 :37-40 —— 已经在 typing 的站就是这个标记本身：
        // 流式下同一处会被扫到多次，定名而不是再开一站。
        let draft = scan_workflow_draft(r#"phase("plan") phase("ver"#);
        assert_eq!(draft.phases.len(), 2, "不新开一站");

        let named = scan_workflow_draft(r#"phase("plan") phase("verify")"#);
        assert_eq!(names(&named), vec!["plan", "verify"]);
        assert!(!named.phases[1].typing, "闭合后不再是 typing");
    }

    #[test]
    fn unclosed_agent_is_not_counted() {
        // 真源 :48 —— agent 必须闭合且名字非空。
        let draft = scan_workflow_draft(r#"agent("worker"#);
        assert!(draft.agents.is_empty(), "没写完的名字不进名册");
        let blank = scan_workflow_draft(r#"agent("")"#);
        assert!(blank.agents.is_empty(), "空名不进名册");
    }

    // ── 草稿模型 ──

    #[test]
    fn draft_model_has_only_stations_and_faint_rails() {
        // ★真源 :6-7 —— 只有站与淡墨轨道段，没有药丸、没有弧、没有状态。
        let draft = scan_workflow_draft(
            r#"phase("plan") agent("甲") phase("implement") agent("乙") phase("review")"#,
        );
        let model = draft_timeline(&draft);
        assert!(!model.live, "草稿不是 live");
        assert_eq!(model.stations.len(), 3);
        assert!(model.arcs.is_empty(), "没有弧");
        assert!(model.bands.is_empty(), "没有带");
        assert_eq!(model.rails.len(), 2, "三站 → 两段轨道");
        assert!(model.rails.iter().all(|r| r.ink == TimelineInk::Faint));
        assert!(model.running_index.is_none());
        assert_eq!(
            model.draft,
            Some(DraftAgents { agents: 2 }),
            "子代理只计数不画"
        );
        assert!(model.stations.iter().all(|s| s.pills.is_empty()));
        assert!(model.stations.iter().all(|s| s.status.is_none()));
        assert!(model.stations.iter().all(|s| !s.visited));
        assert!(model.stations.iter().all(|s| s.rounds == 0));
        assert!(model.stations.iter().all(|s| !s.on_loop));
        assert!(model.stations.iter().all(|s| s.track == 0));
    }

    #[test]
    fn draft_station_ids_are_indexed() {
        let draft = scan_workflow_draft(r#"phase("plan") phase("build")"#);
        let model = draft_timeline(&draft);
        let ids: Vec<&str> = model.stations.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["draft:0", "draft:1"]);
        assert_eq!(model.stations[0].naming.name.as_deref(), Some("plan"));
        assert_eq!(model.stations[1].naming.id, "draft:1");
    }

    #[test]
    fn draft_typing_flag_carries_through() {
        let draft = scan_workflow_draft(r#"phase("plan") phase("ver"#);
        let model = draft_timeline(&draft);
        assert!(!model.stations[0].typing);
        assert!(model.stations[1].typing, "正在写的那一站");
    }

    #[test]
    fn empty_script_gives_an_empty_draft() {
        let draft = scan_workflow_draft("");
        assert!(draft.phases.is_empty());
        assert!(draft.agents.is_empty());
        let model = draft_timeline(&draft);
        assert!(model.stations.is_empty());
        assert!(model.rails.is_empty());
        assert_eq!(model.draft, Some(DraftAgents { agents: 0 }));
    }

    #[test]
    fn rail_endpoints_follow_station_indexes() {
        let draft = scan_workflow_draft(r#"phase("a") phase("b") phase("c")"#);
        let model = draft_timeline(&draft);
        let pairs: Vec<(usize, usize)> = model.rails.iter().map(|r| (r.from, r.to)).collect();
        assert_eq!(pairs, vec![(0, 1), (1, 2)]);
    }
}