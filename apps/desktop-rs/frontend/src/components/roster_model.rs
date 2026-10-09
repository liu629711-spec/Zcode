//! 1:1 翻译 `packages/ui/src/components/workflow-timeline/roster-model.ts`（191 行）。
//!
//! 阶段名册：一站的参与者过了阈值，药丸列换成「钉住的几枚药丸 + 其余」。
//!
//! ★真源 :4-12 注释——这里只做纯的划分——谁被钉住、谁是其余、各状态几人——卡、轮尾摘要、
//! 确认窗与侧板四处同一条规则（不变式 1）。卡把「其余」折成一行「还有 n 个」；
//! 侧板把同一行当门，门后是按状态分组的名单。
//!
//! ★界上列不出来的子代理（`station.unlisted`）没有药丸，但阈值、总数、计数与「还有 n 个」
//! 都得算上它们：一站的数字不随它的子代理离表而缩水，只有行会少。

use crate::components::timeline_model::{TimelinePill, TimelineStation};
use crate::components::workflow_graph::types::StepRunStatus;

/// 参与者 ≤ 这么多枚时仍是药丸列（六枚 = 222 px，已经比名册高）（真源 :15）。
pub const ROSTER_THRESHOLD: usize = 6;
/// 卡上钉住的药丸数：五枚 + 「还有 n 个」一行 = 六枚药丸的高度（真源 :17）。
pub const ROSTER_PINS_CARD: usize = 5;
/// 侧板钉住的药丸数（拉满一列，有地方多说几个名字）（真源 :19）。
pub const ROSTER_PINS_PANE: usize = 5;
/// 「还有 n 个」那一行上叠着的脸数（真源 :21）。
pub const ROSTER_DECK: usize = 3;

/// `RosterCounts`（真源 :23）：按四值状态分桶。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RosterCounts {
    pub done: i64,
    pub failed: i64,
    pub pending: i64,
    pub running: i64,
}

impl RosterCounts {
    fn bump(&mut self, status: StepRunStatus) {
        match status {
            StepRunStatus::Done => self.done += 1,
            StepRunStatus::Failed => self.failed += 1,
            StepRunStatus::Pending => self.pending += 1,
            StepRunStatus::Running => self.running += 1,
        }
    }
}

/// `RosterUnlisted`（真源 :30-34）。
///
/// ★真源 :25-29 注释——表外的那些：归约在界上列不出来的子代理，没有药丸、没有脸、没有行。
/// 名册只**数**它们——一站的数字不能随着它的子代理离表而缩水。`settled` 是其中已知跑完的，
/// `failed ⊆ settled`；剩下的 `actors − settled` 还要跑。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RosterUnlisted {
    pub actors: i64,
    pub settled: i64,
    pub failed: i64,
}

/// `StationRoster`（真源 :38-48）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StationRoster {
    /// 钉住的药丸：asking → running → failed → 按参与者序补位，槽永不空。
    pub pinned: Vec<TimelinePill>,
    /// 其余参与者（没被钉住的）：按参与者序。表外的不在这里——它们没有药丸。
    pub rest: Vec<TimelinePill>,
    /// 全部参与者（钉住的与表外的都算）按状态计数；静态药丸计作 pending。
    pub counts: RosterCounts,
    /// 表外的那些；一条都没少时是零。
    pub unlisted: RosterUnlisted,
    pub total: usize,
}

/// `pillStatusOf`（真源 :51-53）：静态（无 run）与 pending 逐像素相同，计数上也归一类。
pub fn pill_status_of(pill: &TimelinePill) -> StepRunStatus {
    pill.status.unwrap_or(StepRunStatus::Pending)
}

/// `pillInstanceKey`（真源 :56-60）：实例键 `siteId@ordinal`。
///
/// 待答问题按它挂到提问者；合成车道没有。
pub fn pill_instance_key(pill: &TimelinePill) -> Option<String> {
    pill.instance
        .as_ref()
        .map(|i| format!("{}@{}", i.site_id, i.ordinal))
}

/// `rosterCounts`（真源 :62-66）。
pub fn roster_counts(pills: &[TimelinePill]) -> RosterCounts {
    let mut counts = RosterCounts::default();
    for pill in pills {
        counts.bump(pill_status_of(pill));
    }
    counts
}

/// `addUnlisted`（真源 :73-78）：把表外的那些加进一份计数里。
///
/// ★真源 :68-72 注释——只有 `settled` 那部分有结局（失败的进 failed，其余 done），
/// 剩下的还没跑完，进 pending。出生就被拒、或排队时被淘汰的子代理同样在 `actors` 里
/// ——把它们记成 done 是界唯一不能说的那句假话。
fn add_unlisted(mut counts: RosterCounts, unlisted: RosterUnlisted) -> RosterCounts {
    counts.done += unlisted.settled - unlisted.failed;
    counts.failed += unlisted.failed;
    counts.pending += (unlisted.actors - unlisted.settled).max(0);
    counts
}

/// `ATTENTION_ORDER`（真源 :81）：注意力序。
///
/// 「还有 n 个」那一叠脸先露最要紧的；名单的组也按它排。
pub const ATTENTION_ORDER: [StepRunStatus; 4] = [
    StepRunStatus::Failed,
    StepRunStatus::Running,
    StepRunStatus::Pending,
    StepRunStatus::Done,
];

fn attention_rank(status: StepRunStatus) -> usize {
    match status {
        StepRunStatus::Failed => 0,
        StepRunStatus::Running => 1,
        StepRunStatus::Pending => 2,
        StepRunStatus::Done => 3,
    }
}

/// `byAttention`（真源 :90-99）：稳定的注意力排序，同一档内保持参与者序。
fn by_attention(pills: &[TimelinePill]) -> Vec<TimelinePill> {
    let mut indexed: Vec<(usize, &TimelinePill)> = pills.iter().enumerate().collect();
    indexed.sort_by(|(left_i, left), (right_i, right)| {
        attention_rank(pill_status_of(left))
            .cmp(&attention_rank(pill_status_of(right)))
            .then_with(|| left_i.cmp(right_i))
    });
    indexed.into_iter().map(|(_, pill)| pill.clone()).collect()
}

/// `stationRoster` 的选项（真源 :107）。
#[derive(Debug, Clone, Copy)]
pub struct RosterOptions {
    pub pins: usize,
    pub unlisted: Option<RosterUnlisted>,
}

/// `stationRoster`（真源 :106-128）。
///
/// ★真源 :101-105 注释——钉位序 asking → running → failed → 参与者序：
/// 正在跑的一枚不设上限，多少个在跑都钉得满。桶内保持参与者序，
/// 所以一枚 running 的钉位只在它自己停下来时让出——钉位一次只换一枚，不会整列翻。
/// 跑完的 run 既没有 asking 也没有 running 药丸，这条序自己就退化成 failed → 参与者序，
/// 不需要额外的存活输入。
pub fn station_roster(
    pills: &[TimelinePill],
    options: RosterOptions,
) -> Option<StationRoster> {
    let unlisted = options.unlisted.unwrap_or_default();
    // ★真源 :111-113 —— 阈值按**表内 + 表外**判：只剩四枚药丸、身后三百个已淘汰的站，
    // 仍然是一份名册——不然那一行一消失，三百个子代理就在画面上不存在了。
    let total = pills.len() + unlisted.actors.max(0) as usize;
    if total <= ROSTER_THRESHOLD {
        return None;
    }
    let counts = add_unlisted(roster_counts(pills), unlisted);

    let mut pinned: Vec<TimelinePill> = Vec::new();
    let pin = |pill: &TimelinePill, pinned: &mut Vec<TimelinePill>| {
        if pinned.len() < options.pins && !pinned.iter().any(|p| p.key == pill.key) {
            pinned.push(pill.clone());
        }
    };
    for pill in pills.iter().filter(|p| p.asking) {
        pin(pill, &mut pinned);
    }
    for pill in pills
        .iter()
        .filter(|p| pill_status_of(p) == StepRunStatus::Running)
    {
        pin(pill, &mut pinned);
    }
    for pill in pills
        .iter()
        .filter(|p| pill_status_of(p) == StepRunStatus::Failed)
    {
        pin(pill, &mut pinned);
    }
    for pill in pills {
        pin(pill, &mut pinned);
    }

    let rest: Vec<TimelinePill> = pills
        .iter()
        .filter(|p| !pinned.iter().any(|q| q.key == p.key))
        .cloned()
        .collect();
    Some(StationRoster {
        counts,
        pinned,
        rest,
        total,
        unlisted,
    })
}

/// `stationRosterOf`（真源 :134-151）：一站的名册。
///
/// 表外那一格从站上取（`station.unlisted`），所以卡与侧板读的是同一个值，
/// 谁都不必自己去翻 `run.unlistedByPhase`。
pub fn station_roster_of(station: &TimelineStation, pins: usize) -> Option<StationRoster> {
    station_roster(
        &station.pills,
        RosterOptions {
            pins,
            // ★真源 :140 —— 站那一格里的 `nodes_settled` 说的是节点，
            // 名册数的是人：它只进 fraction，不进这里。
            unlisted: station.unlisted.map(|u| RosterUnlisted {
                actors: u.actors,
                failed: u.failed,
                settled: u.settled,
            }),
        },
    )
}

/// `rosterRestCounts`（真源 :154-156）：门关着时那一行的计数行。
///
/// 门后的一切——表内的其余，加上没有药丸的表外那些。
pub fn roster_rest_counts(roster: &StationRoster) -> RosterCounts {
    add_unlisted(roster_counts(&roster.rest), roster.unlisted)
}

/// `RosterMore`（真源 :159-166）：卡上「还有 n 个」那一行的内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterMore {
    /// 没被钉住的参与者数（表外的也算——它们同样在这一行后面）。
    pub count: usize,
    /// 叠着的脸：其余里按注意力序的前几个（failed → running → pending → done）。
    /// 表外的没有脸。
    pub deck: Vec<TimelinePill>,
    /// 藏在这一行后面的 failed 数（钉住的不算，表外失败的算）
    /// ——卡上这一行唯一会说的状态。
    pub failed: i64,
}

/// `rosterMore`（真源 :168-176）。
pub fn roster_more(roster: &StationRoster, deck: usize) -> RosterMore {
    let pinned_failed = roster
        .pinned
        .iter()
        .filter(|p| pill_status_of(p) == StepRunStatus::Failed)
        .count() as i64;
    RosterMore {
        count: roster.rest.len() + roster.unlisted.actors.max(0) as usize,
        deck: by_attention(&roster.rest).into_iter().take(deck).collect(),
        // ★`counts.failed` 已经含表外失败的，减掉钉住的即「这一行后面还藏着几个」。
        failed: roster.counts.failed - pinned_failed,
    }
}

/// `RollGroup`（真源 :179-182）：侧板名单里的一组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollGroup {
    pub status: StepRunStatus,
    pub pills: Vec<TimelinePill>,
}

/// `rosterRoll`（真源 :185-190）：门后的名单。
///
/// 其余按状态分组、组序即注意力序、**空组缺席**。
pub fn roster_roll(roster: &StationRoster) -> Vec<RollGroup> {
    ATTENTION_ORDER
        .iter()
        .map(|status| RollGroup {
            status: *status,
            pills: roster
                .rest
                .iter()
                .filter(|p| pill_status_of(p) == *status)
                .cloned()
                .collect(),
        })
        .filter(|group| !group.pills.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::workflow_graph::lane_name::{LaneNaming, LaneRef};
    use crate::components::workflow_graph::types::{LaneClass, StepRunStatus};

    fn pill(key: &str, status: Option<StepRunStatus>, asking: bool) -> TimelinePill {
        TimelinePill {
            key: key.to_string(),
            lane: LaneRef {
                id: "l1".to_string(),
                naming: LaneNaming {
                    lane_class: Some(LaneClass::Agent),
                    name: None,
                    name_pattern: None,
                    anonymous_index: None,
                },
            },
            lane_class: LaneClass::Agent,
            runtime_name: None,
            avatar_index: None,
            status,
            instance: None,
            slot: None,
            workspace: None,
            step_ids: Vec::new(),
            asking,
        }
    }

    fn roster_of(pills: Vec<TimelinePill>) -> Option<StationRoster> {
        station_roster(
            &pills,
            RosterOptions {
                pins: ROSTER_PINS_CARD,
                unlisted: None,
            },
        )
    }

    // ── 状态归类 ──

    #[test]
    fn static_pill_counts_as_pending() {
        // ★真源 :50 —— 静态（无 run）与 pending 逐像素相同，计数上也归一类。
        assert_eq!(pill_status_of(&pill("a", None, false)), StepRunStatus::Pending);
        let counts = roster_counts(&[pill("a", None, false), pill("b", None, false)]);
        assert_eq!(counts.pending, 2);
        assert_eq!(counts.done, 0);
    }

    #[test]
    fn counts_split_four_states() {
        let counts = roster_counts(&[
            pill("a", Some(StepRunStatus::Done), false),
            pill("b", Some(StepRunStatus::Failed), false),
            pill("c", Some(StepRunStatus::Pending), false),
            pill("d", Some(StepRunStatus::Running), false),
        ]);
        assert_eq!(counts.done, 1);
        assert_eq!(counts.failed, 1);
        assert_eq!(counts.pending, 1);
        assert_eq!(counts.running, 1);
    }

    #[test]
    fn instance_key_joins_site_and_ordinal() {
        // 真源 :56-60 —— 待答问题按它挂到提问者；合成车道没有。
        assert_eq!(pill_instance_key(&pill("a", None, false)), None);
        let mut p = pill("a", None, false);
        p.instance = Some(crate::components::timeline_model::PillInstance {
            site_id: "l1".to_string(),
            ordinal: 2,
            session_id: None,
        });
        assert_eq!(pill_instance_key(&p).as_deref(), Some("l1@2"));
    }

    // ── 阈值 ──

    #[test]
    fn roster_below_threshold_is_none() {
        // 真源 :114 —— 六枚 = 222 px，已经比名册高。
        let six = (0..6)
            .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
            .collect::<Vec<_>>();
        assert!(roster_of(six).is_none(), "六枚仍是药丸列");
        let seven = (0..7)
            .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
            .collect::<Vec<_>>();
        assert!(roster_of(seven).is_some());
    }

    #[test]
    fn threshold_counts_unlisted_actors_too() {
        // ★真源 :111-113 —— 只剩四枚药丸、身后三百个已淘汰的站，仍然是一份名册。
        let pills = (0..4)
            .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
            .collect::<Vec<_>>();
        assert!(roster_of(pills.clone()).is_none(), "表内只有四枚");
        let roster = station_roster(
            &pills,
            RosterOptions {
                pins: ROSTER_PINS_CARD,
                unlisted: Some(RosterUnlisted {
                    actors: 300,
                    settled: 300,
                    failed: 0,
                }),
            },
        )
        .expect("身后三百个已淘汰的站仍是一份名册");
        assert_eq!(roster.total, 304);
        assert_eq!(roster.rest.len(), 0, "表外没有药丸");
    }

    // ── 钉位 ──

    #[test]
    fn asking_wins_the_first_pin() {
        // ★真源 :101-103 —— 钉位序 asking → running → failed → 参与者序。
        let pills = vec![
            pill("z-done", Some(StepRunStatus::Done), false),
            pill("y-failed", Some(StepRunStatus::Failed), false),
            pill("x-running", Some(StepRunStatus::Running), false),
            pill("w-asking", Some(StepRunStatus::Running), true),
            pill("v-done2", Some(StepRunStatus::Done), false),
            pill("u-done3", Some(StepRunStatus::Done), false),
            pill("t-done4", Some(StepRunStatus::Done), false),
        ];
        let roster = roster_of(pills).unwrap();
        let keys: Vec<&str> = roster.pinned.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys[0], "w-asking", "asking 先钉");
        assert_eq!(keys[1], "x-running", "然后 running");
        assert_eq!(keys[2], "y-failed", "然后 failed");
        assert_eq!(keys[3], "z-done", "再按参与者序补位");
        assert_eq!(keys.len(), ROSTER_PINS_CARD);
    }

    #[test]
    fn running_pins_have_no_upper_bound_within_the_cap() {
        // 真源 :102-103 —— 正在跑的一枚不设上限，多少个在跑都钉得满（受 pins 上限约束）。
        let mut pills = vec![pill("w-asking", Some(StepRunStatus::Running), true)];
        for i in 0..9 {
            pills.push(pill(&format!("r{i}"), Some(StepRunStatus::Running), false));
        }
        let roster = station_roster(
            &pills,
            RosterOptions {
                pins: 99,
                unlisted: None,
            },
        )
        .unwrap();
        assert_eq!(roster.pinned.len(), 10, "九个 running + 一个 asking 全部钉住");
        assert!(roster.rest.is_empty());
    }

    #[test]
    fn settled_run_degrades_to_failed_then_participant_order() {
        // ★真源 :104-105 —— 跑完的 run 既没有 asking 也没有 running 药丸，
        // 这条序自己就退化成 failed → 参与者序，不需要额外的存活输入。
        let pills = vec![
            pill("z-done", Some(StepRunStatus::Done), false),
            pill("y-failed", Some(StepRunStatus::Failed), false),
            pill("x-done", Some(StepRunStatus::Done), false),
            pill("w-done", Some(StepRunStatus::Done), false),
            pill("v-done", Some(StepRunStatus::Done), false),
            pill("u-done", Some(StepRunStatus::Done), false),
            pill("t-done", Some(StepRunStatus::Done), false),
        ];
        let roster = roster_of(pills).unwrap();
        let keys: Vec<&str> = roster.pinned.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys[0], "y-failed");
        assert_eq!(keys[1], "z-done", "之后按参与者序");
    }

    #[test]
    fn pin_slot_is_stable_within_a_bucket() {
        // ★真源 :104 —— 桶内保持参与者序，一枚 running 的钉位只在它自己停下来时让出，
        // 不会整列翻。
        let pills = vec![
            pill("r0", Some(StepRunStatus::Running), false),
            pill("d0", Some(StepRunStatus::Done), false),
            pill("d1", Some(StepRunStatus::Done), false),
            pill("d2", Some(StepRunStatus::Done), false),
            pill("d3", Some(StepRunStatus::Done), false),
            pill("d4", Some(StepRunStatus::Done), false),
            pill("d5", Some(StepRunStatus::Done), false),
        ];
        let roster = roster_of(pills).unwrap();
        assert_eq!(roster.pinned[0].key, "r0", "running 占第一槽");
        assert_eq!(roster.pinned[1].key, "d0", "done 从头按序补，不从尾部倒着填");
    }

    #[test]
    fn rest_keeps_participant_order() {
        let pills = (0..8)
            .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
            .collect::<Vec<_>>();
        let roster = roster_of(pills).unwrap();
        let rest: Vec<&str> = roster.rest.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(rest, vec!["p5", "p6", "p7"], "前五枚被钉住，其余按序在后");
    }

    // ── 计数 ──

    #[test]
    fn unlisted_splits_into_settled_and_still_running() {
        // ★真源 :68-72 —— 只有 settled 那部分有结局，剩下的还没跑完。
        let pills = (0..7)
            .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
            .collect::<Vec<_>>();
        let roster = station_roster(
            &pills,
            RosterOptions {
                pins: ROSTER_PINS_CARD,
                unlisted: Some(RosterUnlisted {
                    actors: 10,
                    settled: 6,
                    failed: 2,
                }),
            },
        )
        .unwrap();
        // done += 6 − 2 = 4；failed += 2；pending += 10 − 6 = 4。
        assert_eq!(roster.counts.done, 7 + 4);
        assert_eq!(roster.counts.failed, 2);
        assert_eq!(roster.counts.pending, 4);
    }

    #[test]
    fn unlisted_with_settled_above_actors_is_clamped() {
        // 归约不该产生这种载荷，但真源 `Math.max(0, ...)` 兜住了；
        // Rust 侧同样不能让它把 pending 记成负数。
        let pills = (0..7)
            .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
            .collect::<Vec<_>>();
        let roster = station_roster(
            &pills,
            RosterOptions {
                pins: ROSTER_PINS_CARD,
                unlisted: Some(RosterUnlisted {
                    actors: 2,
                    settled: 5,
                    failed: 0,
                }),
            },
        )
        .unwrap();
        assert_eq!(roster.counts.pending, 0, "不记负数");
        assert_eq!(roster.counts.done, 7 + 5);
    }

    #[test]
    fn rest_counts_add_unlisted_again() {
        // 真源 :154-156 —— 门关着时那一行：表内的其余 + 没有药丸的表外那些。
        let pills = (0..8)
            .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
            .collect::<Vec<_>>();
        let roster = station_roster(
            &pills,
            RosterOptions {
                pins: ROSTER_PINS_CARD,
                unlisted: Some(RosterUnlisted {
                    actors: 3,
                    settled: 1,
                    failed: 1,
                }),
            },
        )
        .unwrap();
        let rest_counts = roster_rest_counts(&roster);
        // rest 三枚都是 done；表外 done += 1 − 1 = 0，failed += 1，pending += 2。
        assert_eq!(rest_counts.done, 3);
        assert_eq!(rest_counts.failed, 1);
        assert_eq!(rest_counts.pending, 2);
    }

    // ── 还有 n 个 ──

    #[test]
    fn more_counts_rest_plus_unlisted() {
        // 真源 :171 —— 没被钉住的参与者数（表外的也算）。
        let pills = (0..8)
            .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
            .collect::<Vec<_>>();
        let roster = station_roster(
            &pills,
            RosterOptions {
                pins: ROSTER_PINS_CARD,
                unlisted: Some(RosterUnlisted {
                    actors: 4,
                    settled: 0,
                    failed: 0,
                }),
            },
        )
        .unwrap();
        let more = roster_more(&roster, ROSTER_DECK);
        assert_eq!(more.count, 3 + 4, "三枚未钉 + 四个表外");
    }

    #[test]
    fn more_deck_shows_the_most_urgent_first() {
        // ★真源 :165 —— 叠着的脸按注意力序（failed → running → pending → done）。
        // 五枚钉位额度先被 asking / running / failed 占满，rest 里三种状态都有。
        let pills = vec![
            pill("f0", Some(StepRunStatus::Failed), false),
            pill("r0", Some(StepRunStatus::Running), false),
            pill("a0", Some(StepRunStatus::Pending), true),
            pill("d0", Some(StepRunStatus::Done), false),
            pill("d1", Some(StepRunStatus::Done), false),
            pill("d2", Some(StepRunStatus::Done), false),
            pill("d3", Some(StepRunStatus::Done), false),
        ];
        let roster = roster_of(pills).unwrap();
        assert_eq!(roster.pinned.len(), ROSTER_PINS_CARD, "五枚额度被 asking/running/failed 占满");
        let more = roster_more(&roster, 3);
        let deck: Vec<&str> = more.deck.iter().map(|p| p.key.as_str()).collect();
        // 钉住 a0(asking) / r0(running) / f0(failed) / d0 / d1；
        // rest 是 d2、d3 —— 状态同档时按参与者序。
        assert_eq!(deck, vec!["d2", "d3"]);
    }

    #[test]
    fn attention_deck_orders_across_states() {
        // 钉位额度只够两枚（asking 一枚 + 其余按参与者序补），
        // 于是 rest 里同时有 running / pending / done，注意力序才看得出来。
        let pills = vec![
            pill("a0", Some(StepRunStatus::Pending), true),
            pill("d0", Some(StepRunStatus::Done), false),
            pill("r0", Some(StepRunStatus::Running), false),
            pill("d1", Some(StepRunStatus::Done), false),
            pill("p0", Some(StepRunStatus::Pending), false),
            pill("d2", Some(StepRunStatus::Done), false),
            pill("d3", Some(StepRunStatus::Done), false),
        ];
        let roster = station_roster(
            &pills,
            RosterOptions {
                pins: 1,
                unlisted: None,
            },
        )
        .unwrap();
        let more = roster_more(&roster, 3);
        let deck: Vec<&str> = more.deck.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(deck, vec!["r0", "p0", "d0"], "running → pending → done");
    }

    #[test]
    fn roll_groups_by_attention_order_and_drops_empty() {
        // ★真源 :184-190 —— 组序即注意力序、空组缺席。
        // 钉位额度只够两枚，rest 里三种状态都有；failed 组空 → 缺席。
        let pills = vec![
            pill("a0", Some(StepRunStatus::Pending), true),
            pill("d0", Some(StepRunStatus::Done), false),
            pill("r0", Some(StepRunStatus::Running), false),
            pill("d1", Some(StepRunStatus::Done), false),
            pill("p0", Some(StepRunStatus::Pending), false),
            pill("d2", Some(StepRunStatus::Done), false),
            pill("d3", Some(StepRunStatus::Done), false),
        ];
        let roster = station_roster(
            &pills,
            RosterOptions {
                pins: 1,
                unlisted: None,
            },
        )
        .unwrap();
        let roll = roster_roll(&roster);
        let statuses: Vec<StepRunStatus> = roll.iter().map(|g| g.status).collect();
        assert_eq!(
            statuses,
            vec![StepRunStatus::Running, StepRunStatus::Pending, StepRunStatus::Done]
        );
        assert_eq!(roll[0].pills[0].key, "r0");
        assert_eq!(roll[1].pills[0].key, "p0");
    }

    #[test]
    fn more_failed_excludes_pinned() {
        // ★真源 :166 / :173 —— `counts.failed` 含表外失败的，减掉钉住的
        // 即「这一行后面还藏着几个」。
        let pills = vec![
            pill("f0", Some(StepRunStatus::Failed), false),
            pill("f1", Some(StepRunStatus::Failed), false),
            pill("d0", Some(StepRunStatus::Done), false),
            pill("d1", Some(StepRunStatus::Done), false),
            pill("d2", Some(StepRunStatus::Done), false),
            pill("d3", Some(StepRunStatus::Done), false),
            pill("d4", Some(StepRunStatus::Done), false),
        ];
        let roster = station_roster(
            &pills,
            RosterOptions {
                pins: ROSTER_PINS_CARD,
                unlisted: Some(RosterUnlisted {
                    actors: 2,
                    settled: 2,
                    failed: 1,
                }),
            },
        )
        .unwrap();
        let more = roster_more(&roster, ROSTER_DECK);
        // counts.failed = 2（表内）+ 1（表外）= 3；钉住 2 枚 → 后面还藏 1 个。
        assert_eq!(more.failed, 1);
    }

    // ── 名单 ──

    #[test]
    fn roll_keeps_participant_order_inside_a_group() {
        let pills = (0..8)
            .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
            .collect::<Vec<_>>();
        let roster = roster_of(pills).unwrap();
        let roll = roster_roll(&roster);
        assert_eq!(roll.len(), 1, "全是 done → 只有一组");
        let keys: Vec<&str> = roll[0].pills.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys, vec!["p5", "p6", "p7"]);
    }

    // ── 站级入口 ──

    #[test]
    fn station_roster_of_reads_unlisted_from_the_station() {
        // ★真源 :131-133 —— 表外那一格从站上取，卡与侧板读的是同一个值。
        let mut station = crate::components::timeline_model::TimelineStation {
            id: "p1".to_string(),
            naming: crate::components::workflow_graph::phase_name::PhaseNaming {
                id: "p1".to_string(),
                name: None,
            },
            pills: (0..7)
                .map(|i| pill(&format!("p{i}"), Some(StepRunStatus::Done), false))
                .collect(),
            status: None,
            visited: false,
            rounds: 0,
            on_loop: false,
            track: 0,
            fraction: None,
            unlisted: None,
            typing: false,
        };
        let plain = station_roster_of(&station, ROSTER_PINS_CARD).unwrap();
        assert_eq!(plain.total, 7);

        station.unlisted = Some(crate::components::station_observation::StationUnlisted {
            actors: 5,
            settled: 5,
            failed: 1,
            nodes_settled: 99,
        });
        let with_unlisted = station_roster_of(&station, ROSTER_PINS_CARD).unwrap();
        assert_eq!(with_unlisted.total, 12, "表外五个也算进总数");
        assert_eq!(with_unlisted.unlisted.failed, 1);
        // ★`nodes_settled` 说的是节点，名册数的是人：它不进这里。
        assert_eq!(with_unlisted.total, 7 + 5);
    }

    #[test]
    fn attention_order_is_failed_running_pending_done() {
        assert_eq!(
            ATTENTION_ORDER,
            [
                StepRunStatus::Failed,
                StepRunStatus::Running,
                StepRunStatus::Pending,
                StepRunStatus::Done
            ]
        );
        assert_eq!(ROSTER_THRESHOLD, 6);
        assert_eq!(ROSTER_PINS_CARD, 5);
        assert_eq!(ROSTER_PINS_PANE, 5);
        assert_eq!(ROSTER_DECK, 3);
    }
}