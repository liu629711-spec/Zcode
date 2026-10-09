//! 假数据渲染画廊（Rust 侧自造，**不是** packages/ui 的真源对照文件）。
//!
//! 为什么要它：工程里 1400+ 单测全是纯函数断言，**没有一个测试能渲染**，
//! 而 `trunk serve` 起来没有 Tauri 后端和 agent 数据，工具卡平时根本没有
//! 验证面——改动只能靠"照着源码写"，不能靠"看到它对了"。
//!
//! 这里用写死的 props 直接把 `ToolLayoutComponent` 摆出来，专挑今天改过的那些
//! 面：摘要行类名、diff 计数归属（应在行内而非行外）、增删计数文本、展开态换文案、
//! 状态位节点、文件 chip。浏览器入口是 `http://127.0.0.1:1420/?fixture=1`。
//!
//! 产品路径不经过这里（`main.rs` 按 query 分流），也不碰 IPC。

use leptos::prelude::*;

use crate::ToolCallBlocks::ToolLayout::{FileChipData, ToolLayoutComponent, ToolLayoutProps};

/// 一个样例：标题 + 对应的卡。
#[component]
fn Sample(
    // #[prop(into)] 让调用处能直接写字符串字面量，不必每处 .to_string()。
    #[prop(into)] title: String,
    props: ToolLayoutProps,
) -> impl IntoView {
    let no_icon: Option<ChildrenFn> = None;
    let body: ChildrenFn = std::sync::Arc::new(|| {
        view! { <div class="px-2 py-1 text-ui-xs text-foreground-subtlest">"这里是折叠内容体"</div> }
            .into_any()
    });
    view! {
        <section class="mb-6">
            <h2 class="mb-1 font-mono text-ui-xs uppercase text-foreground-subtlest">{title}</h2>
            <div class="rounded border border-border bg-background-alt p-2">
                <ToolLayoutComponent
                    props=props
                    icon_view=no_icon
                    render_content=Some(body)
                />
            </div>
        </section>
    }
}

#[component]
pub fn FixtureGallery() -> impl IntoView {
    // 1) 运行态：kind 文案走扫光，摘要行动画已请求（滚轮未接，静态渲染）。
    let running = ToolLayoutProps {
        tool_id: "fx-running".into(),
        kind_label: Some("正在读取".into()),
        primary_text: Some("src/main.rs".into()),
        secondary_text: Some("第 1-40 行".into()),
        is_running: Some(true),
        animate_summary_content: Some(true),
        ..Default::default()
    };

    // 2) 展开态换文案 + diff 计数只在收起时出现。
    //    点开这张卡应当看到：摘要主文案换成「已编辑…」，次文本消失，+3/-1 消失，
    //    箭头转 90 度——这几样今天才修好，之前整行点了没反应。
    let expandable = ToolLayoutProps {
        tool_id: "fx-expand".into(),
        kind_label: Some("编辑".into()),
        primary_text: Some("收起态：改了 src/app.rs".into()),
        secondary_text: Some("收起态次文本".into()),
        expanded_primary_text: Some("展开态：摘要文案应换成这一行".into()),
        hide_secondary_text_when_open: Some(true),
        title: Some("收起态标题".into()),
        expanded_title: Some("展开态标题".into()),
        diff_count: Some((3, 1)),
        hide_diff_count_when_open: Some(true),
        status_label: Some("已完成".into()),
        show_status_label: Some(true),
        ..Default::default()
    };

    // 3) 失败态 + 状态词 tooltip（复制按钮应紧跟状态词）。
    let failed = ToolLayoutProps {
        tool_id: "fx-failed".into(),
        kind_label: Some("执行".into()),
        primary_text: Some("cargo test".into()),
        status_label: Some("失败".into()),
        status_tooltip: Some("命令退出码 101，2 个断言失败。".into()),
        show_failure_status: Some(true),
        ..Default::default()
    };

    // 4) 文件 chip 形态的 primaryText（真源是 ReactNode，这里走 FileChipData）。
    let chip = ToolLayoutProps {
        tool_id: "fx-chip".into(),
        kind_label: Some("读取".into()),
        primary_file_chip: Some(FileChipData {
            path: "packages/ui/src/ToolCallBlocks/ToolLayout.rs".into(),
            file_name: "ToolLayout.rs".into(),
            icon_src: "/material-icons/ts.svg".into(),
            clickable: true,
        }),
        status_label: Some("已完成".into()),
        show_status_label: Some(true),
        ..Default::default()
    };

    // 5) 状态位是节点（圆点 + 词），不是纯文本。
    let dot_and_word: ChildrenFn = std::sync::Arc::new(|| {
        view! {
            <span class="inline-flex items-center gap-1.5">
                <span class="size-2 flex-none rounded-full bg-amber-500"></span>
                <span>"等待确认"</span>
            </span>
        }
        .into_any()
    });
    let status_node = ToolLayoutProps {
        tool_id: "fx-status-node".into(),
        kind_label: Some("提问".into()),
        primary_text: Some("要改哪个文件？".into()),
        status_label_view: Some(dot_and_word),
        show_status_label: Some(true),
        can_toggle: Some(false),
        ..Default::default()
    };

    view! {
        <main class="mx-auto max-w-3xl p-6 text-ui-base">
            <h1 class="mb-1 text-ui-lg font-medium">"工具卡假数据画廊"</h1>
            <p class="mb-6 text-ui-xs text-foreground-subtlest">
                {"不经 Tauri、不读后端数据，只为在真浏览器里核对渲染结果。"}
            </p>
            <Sample title="1 · 运行态" props=running />
            <Sample title="2 · 展开态换文案 + diff 计数收起时才出现" props=expandable />
            <Sample title="3 · 失败态 + 状态词 tooltip" props=failed />
            <Sample title="4 · 文件 chip 形态主文本" props=chip />
            <Sample title="5 · 状态位是节点（圆点+词），不可折叠" props=status_node />
        </main>
    }
}
