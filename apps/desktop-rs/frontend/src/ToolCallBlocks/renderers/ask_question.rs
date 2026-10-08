//! 1:1 翻译 `packages/ui/src/lib/askUserQuestion.ts`（349 行）+
//! `packages/ui/src/ToolCallBlocks/renderers/ask-question.tsx`（92 行）。
//!
//! 问询卡：问题列表 + 回答文本（多路读取）。问答数据是双向兼容的——
//! ZCode Agent 的单题输入与交互请求的多题输入共用同一展示管线
//! （真源 :85-86 注释）。
//!
//! 文案（zh-CN.ts:5938-5942）：asking「正在询问」/ asked「已询问」/
//! questionsCount「{count} 个问题」/ noAnswerProvided「未提供回答」/
//! autoContinued「未回答，已自动继续」。

use leptos::prelude::*;
use serde_json::{Map, Value};

/// 回答映射（真源 `ZCodeUserQuestionAnswers`，:1）。
pub type QuestionAnswers = Map<String, Value>;

/// `AskUserQuestionType`（真源 :3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionType {
    Single,
    Multiple,
}

/// `AskUserQuestionOption`（真源 :5-11）。
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionOption {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub placeholder: Option<String>,
    pub requires_input: bool,
}

/// `AskUserQuestionItem`（真源 :13-19）。
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionItem {
    pub id: String,
    pub question: String,
    pub qtype: QuestionType,
    pub options: Vec<QuestionOption>,
    pub custom_input: Option<QuestionOption>,
}

/// `AskUserQuestionData`（真源 :21-24）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuestionData {
    pub questions: Vec<QuestionItem>,
    pub answers: Option<QuestionAnswers>,
}

/// `CUSTOM_INPUT_FLAGS`（真源 :32-38）。
const CUSTOM_INPUT_FLAGS: [&str; 5] = [
    "requiresInput",
    "customInput",
    "isCustom",
    "freeText",
    "allowFreeText",
];

fn is_plain_record(value: &Value) -> bool {
    value.is_object()
}

/// `readString`（真源 :46-54）——返回**未 trim** 的原始值。
fn read_string(record: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = record.get(*key).and_then(|v| v.as_str()) {
            if !value.trim().is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// `readBoolean`（真源 :56-58）。
fn read_boolean(record: &Value, keys: &[&str]) -> bool {
    keys.iter()
        .any(|key| record.get(*key).and_then(|v| v.as_bool()) == Some(true))
}

/// `normalizeOption`（真源 :60-88）。
pub fn normalize_option(value: &Value, index: usize) -> Option<QuestionOption> {
    if let Some(s) = value.as_str() {
        return Some(QuestionOption {
            id: format!("option-{index}"),
            label: s.to_string(),
            description: None,
            placeholder: None,
            requires_input: false,
        });
    }
    if !is_plain_record(value) {
        return None;
    }

    let label = read_string(value, &["label", "name", "title", "text", "value"])
        .unwrap_or_else(|| value.to_string());
    let id =
        read_string(value, &["id", "optionId", "value", "key"]).unwrap_or_else(|| label.clone());
    let placeholder = read_string(
        value,
        &[
            "placeholder",
            "customInputPlaceholder",
            "freeTextPlaceholder",
            "inputPlaceholder",
        ],
    );

    Some(QuestionOption {
        id,
        label,
        description: read_string(value, &["description", "detail", "help", "hint"]),
        // 真源 :84-86 —— 有 placeholder 也算「要求输入」。
        requires_input: read_boolean(value, &CUSTOM_INPUT_FLAGS) || placeholder.is_some(),
        placeholder,
    })
}

/// `CUSTOM_INPUT_LABEL_PATTERN`（真源 :39-40）——父选择项暗示自定义输入。
fn is_implicit_custom_input_option(option: &QuestionOption) -> bool {
    custom_input_label_re().is_match(&option.label)
}

fn custom_input_label_re() -> &'static regex_lite::Regex {
    static RE: std::sync::OnceLock<regex_lite::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex_lite::Regex::new(
            r"(?i)(其他|其它|自定义|自行|自己|补充|填写|填入|输入|other|custom|free\s*text|specify|write\s*in)",
        )
        .expect("自定义输入标签正则")
    })
}

/// `readQuestions`（真源 :95-107）。
fn read_questions(input: &Value) -> Vec<Value> {
    if !is_plain_record(input) {
        return Vec::new();
    }
    if let Some(questions) = input.get("questions").and_then(|q| q.as_array()) {
        return questions.clone();
    }
    // 单题输入：question 是字符串且有 options。
    if input.get("question").and_then(|q| q.as_str()).is_some()
        && input.get("options").and_then(|o| o.as_array()).is_some()
    {
        return vec![input.clone()];
    }
    Vec::new()
}

/// `readAnswers`（真源 :109-116）。
fn read_answers(input: &Value) -> Option<QuestionAnswers> {
    if !is_plain_record(input) {
        return None;
    }
    input
        .get("answers")
        .filter(|a| is_plain_record(a))
        .and_then(|a| a.as_object().cloned())
}

/// `parseJsonRecord`（真源 :118-130）。
fn parse_json_record(output: &Value) -> Option<Value> {
    if is_plain_record(output) {
        return Some(output.clone());
    }
    let s = output.as_str().filter(|s| !s.trim().is_empty())?;
    serde_json::from_str::<Value>(s)
        .ok()
        .filter(|v| is_plain_record(v))
}

/// `readNestedAskUserQuestionAnswers`（真源 :132-162）。
fn read_nested_answers(input: &Value) -> Option<QuestionAnswers> {
    let record = parse_json_record(input)?;
    if let Some(direct) = read_answers(&record) {
        return Some(direct);
    }
    if let Some(content) = record.get("content") {
        if let Some(nested) = read_nested_answers(content) {
            return Some(nested);
        }
        if let Some(items) = content.as_array() {
            for item in items {
                if let Some(nested) = read_nested_answers(item) {
                    return Some(nested);
                }
            }
        }
    }
    if let Some(raw_output) = record.get("rawOutput") {
        if let Some(nested) = read_nested_answers(raw_output) {
            return Some(nested);
        }
    }
    record.get("output").and_then(read_nested_answers)
}

/// `parseZCodeAskUserQuestionOutput`（真源 :164-193）：answered/answered_custom 结果。
fn parse_zcode_output(output: &Value, input: &Value) -> Option<QuestionAnswers> {
    let output_record = parse_json_record(output)?;
    let data = normalize_ask_user_question_input(input);
    let question = data.questions.first()?;

    let ty = output_record.get("type").and_then(|t| t.as_str());
    let raw_answer = match ty {
        Some("answered") => output_record.get("selected"),
        Some("answered_custom") => output_record.get("text"),
        _ => None,
    };
    let raw_answer = raw_answer.and_then(|a| a.as_str())?;

    let mut answers = QuestionAnswers::new();
    answers.insert(
        question.question.clone(),
        Value::String(raw_answer.to_string()),
    );
    Some(answers)
}

/// `normalizeQuestion`（真源 :195-245）。
pub fn normalize_question(value: &Value, index: usize) -> Option<QuestionItem> {
    if !is_plain_record(value) {
        return None;
    }
    let question = read_string(value, &["question", "prompt", "label", "title", "text"])?;

    let raw_options = value
        .get("options")
        .and_then(|o| o.as_array())
        .cloned()
        .unwrap_or_default();
    let options: Vec<QuestionOption> = raw_options
        .iter()
        .enumerate()
        .filter_map(|(i, option)| normalize_option(option, i))
        .collect();

    let question_placeholder = read_string(
        value,
        &[
            "customInputPlaceholder",
            "freeTextPlaceholder",
            "inputPlaceholder",
        ],
    );
    let last_option = options.last();
    let last_option_is_custom_input = last_option
        .is_some_and(|last| last.requires_input || is_implicit_custom_input_option(last));
    let custom_input = if last_option_is_custom_input {
        // 真源 :211-218 —— 把末项升级为自定义输入（placeholder 回落 label）。
        last_option.map(|last| QuestionOption {
            id: last.id.clone(),
            label: last.label.clone(),
            description: last.description.clone(),
            placeholder: Some(
                last.placeholder
                    .clone()
                    .unwrap_or_else(|| last.label.clone()),
            ),
            requires_input: true,
        })
    } else if let Some(placeholder) = question_placeholder {
        Some(QuestionOption {
            id: format!("custom-{index}"),
            label: placeholder.clone(),
            description: None,
            placeholder: Some(placeholder),
            requires_input: true,
        })
    } else {
        None
    };
    let normalized_options = if last_option_is_custom_input {
        options[..options.len() - 1].to_vec()
    } else {
        options
    };

    let qtype = if value.get("multiple").and_then(|v| v.as_bool()) == Some(true)
        || value.get("multiSelect").and_then(|v| v.as_bool()) == Some(true)
        || matches!(
            value.get("type").and_then(|v| v.as_str()),
            Some("multiple") | Some("multi")
        ) {
        QuestionType::Multiple
    } else {
        QuestionType::Single
    };

    Some(QuestionItem {
        id: read_string(value, &["id", "key", "name"])
            .unwrap_or_else(|| format!("question-{index}")),
        question,
        qtype,
        options: normalized_options,
        custom_input,
    })
}

/// `normalizeAskUserQuestionInput`（真源 :247-254）。
pub fn normalize_ask_user_question_input(input: &Value) -> QuestionData {
    QuestionData {
        questions: read_questions(input)
            .iter()
            .enumerate()
            .filter_map(|(i, q)| normalize_question(q, i))
            .collect(),
        answers: read_answers(input),
    }
}

/// `readAskUserQuestionInput`（真源 :256-271）。
pub fn read_ask_user_question_input(input: &Value, raw: &Value) -> Value {
    if !read_questions(input).is_empty() {
        return input.clone();
    }
    if is_plain_record(raw) {
        if let Some(raw_input) = raw.get("rawInput") {
            if !read_questions(raw_input).is_empty() {
                return raw_input.clone();
            }
        }
        if let Some(raw_input) = raw.get("input") {
            if !read_questions(raw_input).is_empty() {
                return raw_input.clone();
            }
        }
    }
    input.clone()
}

/// `readAskUserQuestionAnswers`（真源 :273-313）。
pub fn read_ask_user_question_answers(
    input: &Value,
    output: &Value,
    raw: &Value,
) -> Option<QuestionAnswers> {
    if let Some(nested) = read_nested_answers(output) {
        return Some(nested);
    }
    let question_input = read_ask_user_question_input(input, raw);
    if let Some(parsed) = parse_zcode_output(output, &question_input) {
        return Some(parsed);
    }
    if is_plain_record(input) {
        if let Some(answers) = read_answers(input) {
            return Some(answers);
        }
    }
    if is_plain_record(raw) {
        if let Some(raw_output) = raw.get("rawOutput") {
            if let Some(nested) = read_nested_answers(raw_output) {
                return Some(nested);
            }
            if let Some(parsed) = parse_zcode_output(raw_output, &question_input) {
                return Some(parsed);
            }
        }
        if let Some(raw_output) = raw.get("output") {
            if let Some(nested) = read_nested_answers(raw_output) {
                return Some(nested);
            }
            if let Some(parsed) = parse_zcode_output(raw_output, &question_input) {
                return Some(parsed);
            }
        }
        if let Some(raw_content) = raw.get("content") {
            if let Some(nested) = read_nested_answers(raw_content) {
                return Some(nested);
            }
        }
    }
    None
}

/// `getAskUserQuestionAnswerText`（真源 :315-348）。
pub fn get_ask_user_question_answer_text(
    question: &QuestionItem,
    answers: Option<&QuestionAnswers>,
    no_answer_text: &str,
) -> String {
    let value = answers
        .and_then(|a| a.get(&question.question))
        .or_else(|| answers.and_then(|a| a.get(&question.id)));
    match value {
        Some(Value::Array(items)) => {
            let values: Vec<String> = items
                .iter()
                .map(|item| {
                    item.as_str()
                        .unwrap_or(&item.to_string())
                        .trim()
                        .to_string()
                })
                .filter(|s| !s.is_empty())
                .collect();
            if values.is_empty() {
                no_answer_text.to_string()
            } else {
                values.join("，")
            }
        }
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                no_answer_text.to_string()
            } else {
                s.clone()
            }
        }
        Some(v) if !v.is_null() => v.as_str().unwrap_or(&v.to_string()).to_string(),
        _ => no_answer_text.to_string(),
    }
}

/// `AskQuestionToolCallBlock` 的 props。
#[derive(Debug, Clone)]
pub struct AskQuestionBlockProps {
    pub tool_id: String,
    pub input: Value,
    pub output: Value,
    pub raw: Value,
    pub status: Option<String>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub title: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
}

/// `AskQuestionToolCallBlock`（真源 ask-question.tsx:17-92）。
#[component]
pub fn AskQuestionToolCallBlock(props: AskQuestionBlockProps) -> impl IntoView {
    let question_input = read_ask_user_question_input(&props.input, &props.raw);
    let data = normalize_ask_user_question_input(&question_input);
    // 真源 :23 —— `readAskUserQuestionAnswers(toolCall)`：output 优先，input 兜底。
    let answers = read_ask_user_question_answers(&props.input, &props.output, &props.raw)
        .or(data.answers.clone());
    let is_failed = props.status.as_deref() == Some("failed");
    // 真源 :26-30 —— 非运行、非失败、answers 是空对象 → 「已自动继续」。
    let was_automatically_continued =
        !props.is_running && !is_failed && answers.as_ref().is_some_and(|a| a.is_empty());
    let no_answer_text = if was_automatically_continued {
        "未回答，已自动继续"
    } else {
        "未提供回答"
    };

    let question_count = data.questions.len();
    let kind_label = if props.is_running {
        "正在询问"
    } else {
        "已询问"
    };
    let secondary_text = if props.is_running {
        None
    } else if was_automatically_continued {
        Some(no_answer_text.to_string())
    } else {
        Some(format!("{question_count} 个问题"))
    };

    let questions_for_render: Vec<(String, String, String)> = data
        .questions
        .iter()
        .map(|q| {
            (
                q.id.clone(),
                q.question.clone(),
                get_ask_user_question_answer_text(q, answers.as_ref(), no_answer_text),
            )
        })
        .collect();
    let questions_empty = question_count == 0;
    let show_content = !props.is_running || is_failed;
    let content = move || {
        if !show_content {
            return ().into_any();
        }
        let questions = questions_for_render.clone();
        let empty_text = no_answer_text.to_string();
        view! {
            <div class="ml-2 space-y-2 border-l border-border pl-3.5">
                {questions
                    .into_iter()
                    .map(|(id, question, answer)| {
                        view! {
                            <div class="space-y-1" data-question-id=id>
                                <p class="text-ui-base font-medium leading-5 text-foreground">
                                    {question}
                                </p>
                                <p class="text-ui-base leading-5 text-foreground-subtle">{answer}</p>
                            </div>
                        }
                    })
                    .collect_view()}
                {questions_empty.then(|| view! {
                    <p class="text-ui-base leading-5 text-foreground-subtle">{empty_text}</p>
                })}
            </div>
        }
        .into_any()
    };

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                icon: None,
                show_icon: Some(props.show_icon),
                can_toggle: Some(!props.is_running),
                force_open: Some(false),
                auto_open: Some(false),
                kind_label: Some(kind_label.to_string()),
                source_label: props.source_label.clone(),
                primary_text: None,
                secondary_text,
                status_label: is_failed.then(|| props.status_label.clone()).flatten(),
                status_tooltip: is_failed.then(|| props.error_text.clone()).flatten(),
                show_failure_status: Some(is_failed),
                is_running: Some(props.is_running),
                title: Some(props.title.clone().unwrap_or_else(|| "AskUserQuestion".to_string())),
                ..Default::default()
            }
            render_content=show_content.then(|| std::sync::Arc::new(content) as ChildrenFn)
            icon_view=Some(std::sync::Arc::new(|| {
                view! {
                    <span class="size-4 flex-none text-foreground-subtle">
                        // CircleHelpIcon（lucide）。
                        <svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="12" cy="12" r="10"></circle>
                            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path>
                            <path d="M12 17h.01"></path>
                        </svg>
                    </span>
                }
                .into_any()
            }))
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn options_from_strings_and_records() {
        // 字符串选项。
        let opt = normalize_option(&json!("选项甲"), 0).unwrap();
        assert_eq!(opt.id, "option-0");
        assert!(!opt.requires_input);
        // 记录选项：label/id/description。
        let opt = normalize_option(
            &json!({"id": "a", "label": "甲", "description": "说明", "placeholder": "请填"}),
            3,
        )
        .unwrap();
        assert_eq!(opt.id, "a");
        assert_eq!(opt.description.as_deref(), Some("说明"));
        // 有 placeholder → requires_input（真源 :85）。
        assert!(opt.requires_input);
    }

    #[test]
    fn single_question_input_is_wrapped() {
        // 真源 :103-105 —— 单题输入（question + options）。
        let data = normalize_ask_user_question_input(&json!({
            "question": "选哪个？",
            "options": ["甲", "乙"],
        }));
        assert_eq!(data.questions.len(), 1);
        assert_eq!(data.questions[0].id, "question-0");
        assert_eq!(data.questions[0].options.len(), 2);
    }

    #[test]
    fn last_custom_option_upgraded_and_removed() {
        // 真源 :206-224 —— 末项是「其他」→ 升级为 customInput 并从 options 移除。
        let data = normalize_ask_user_question_input(&json!({
            "questions": [{
                "question": "Q",
                "options": ["甲", {"label": "其他", "placeholder": "自填"}],
            }]
        }));
        let q = &data.questions[0];
        assert_eq!(q.options.len(), 1);
        let custom = q.custom_input.as_ref().unwrap();
        assert_eq!(custom.label, "其他");
        assert!(custom.requires_input);
        // 隐式标签匹配（「其他」）。
        assert!(is_implicit_custom_input_option(&QuestionOption {
            id: "x".into(),
            label: "其他".into(),
            description: None,
            placeholder: None,
            requires_input: false,
        }));
    }

    #[test]
    fn question_placeholder_becomes_custom_input() {
        // 真源 :219-225 —— 无末项自定义时，question 级 placeholder 造一条。
        let data = normalize_ask_user_question_input(&json!({
            "questions": [{
                "question": "Q",
                "options": ["甲"],
                "inputPlaceholder": "自填答案",
            }]
        }));
        let q = &data.questions[0];
        assert_eq!(q.options.len(), 1);
        assert_eq!(q.custom_input.as_ref().unwrap().id, "custom-0");
    }

    #[test]
    fn multiple_detection() {
        // 真源 :227-233 —— multiple/multiSelect/type 三种。
        for input in [
            json!({"questions":[{"question":"Q","multiple":true,"options":[]}]}),
            json!({"questions":[{"question":"Q","multiSelect":true,"options":[]}]}),
            json!({"questions":[{"question":"Q","type":"multi","options":[]}]}),
        ] {
            let data = normalize_ask_user_question_input(&input);
            assert_eq!(data.questions[0].qtype, QuestionType::Multiple);
        }
        let single = normalize_ask_user_question_input(&json!({
            "questions": [{"question": "Q", "options": []}]
        }));
        assert_eq!(single.questions[0].qtype, QuestionType::Single);
    }

    #[test]
    fn answers_read_ladder() {
        // output 的 answered_custom（真源 :178-190）。
        let input = json!({"questions": [{"question": "选哪个", "options": ["甲"]}]});
        let output = json!({"type": "answered_custom", "text": "自定义回答"});
        let answers = read_ask_user_question_answers(&input, &output, &json!({})).unwrap();
        assert_eq!(
            answers.get("选哪个").and_then(|v| v.as_str()),
            Some("自定义回答")
        );
        // input.answers 回落。
        let input = json!({
            "questions": [{"question": "Q", "options": ["甲"]}],
            "answers": {"Q": "甲"}
        });
        let answers = read_ask_user_question_answers(&input, &json!({}), &json!({})).unwrap();
        assert_eq!(answers.get("Q").and_then(|v| v.as_str()), Some("甲"));
        // 无任何 answers。
        assert!(
            read_ask_user_question_answers(
                &json!({"questions": [{"question": "Q", "options": []}]}),
                &json!({}),
                &json!({})
            )
            .is_none()
        );
    }

    #[test]
    fn answer_text_formats() {
        let question = QuestionItem {
            id: "q1".into(),
            question: "Q".into(),
            qtype: QuestionType::Single,
            options: vec![],
            custom_input: None,
        };
        let mut answers = QuestionAnswers::new();
        // 数组 → 中文逗号连接。
        answers.insert("Q".into(), json!(["甲", "乙"]));
        assert_eq!(
            get_ask_user_question_answer_text(&question, Some(&answers), "未提供回答"),
            "甲，乙"
        );
        // 空数组 → noAnswer。
        answers.insert("Q".into(), json!([]));
        assert_eq!(
            get_ask_user_question_answer_text(&question, Some(&answers), "未提供回答"),
            "未提供回答"
        );
        // 空字符串 → noAnswer。
        answers.insert("Q".into(), json!("  "));
        assert_eq!(
            get_ask_user_question_answer_text(&question, Some(&answers), "未提供回答"),
            "未提供回答"
        );
        // 题面 key 缺失时回落到 id。
        answers.remove("Q");
        answers.insert("q1".into(), json!("按 id 命中"));
        assert_eq!(
            get_ask_user_question_answer_text(&question, Some(&answers), "未提供回答"),
            "按 id 命中"
        );
        // 无 answers。
        assert_eq!(
            get_ask_user_question_answer_text(&question, None, "未提供回答"),
            "未提供回答"
        );
    }
}
