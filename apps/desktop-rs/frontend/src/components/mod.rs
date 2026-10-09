//! 1:1 翻译 `packages/ui/src/components/` 下的共享组件与逻辑。
//!
//! 目前迁入的是 workflow-timeline 的设置轮措辞（被 toolCall 的多张卡共用）。

#![allow(non_snake_case)]

pub mod draft_scan;
pub mod roster_model;
pub mod station_observation;
pub mod subagent_model_label;
pub mod timeline_bands;
pub mod timeline_geometry;
pub mod timeline_ledge;
pub mod timeline_model;
pub mod timeline_summary;
pub mod useTimelineViewport;
pub mod useTypewriter;
pub mod workflow_face_motion;
pub mod workflowIcons;
pub mod workflowRunSettings;
pub mod workflowSettingsChange;
pub mod workflow_graph;
pub mod WorkflowAgentFace;
pub mod WorkflowAgentPill;
pub mod WorkflowArtifactIndex;
pub mod WorkflowArtifactPill;
pub mod WorkflowArtifactRow;
pub mod WorkflowArtifactStrip;
pub mod WorkflowArtifactTile;
pub mod WorkflowCardChrome;
pub mod WorkflowCompletionArtifacts;
pub mod WorkflowCompletionCard;
pub mod WorkflowMarchLight;
pub mod WorkflowMoreRow;
pub mod WorkflowRoll;
pub mod WorkflowRosterParts;
pub mod WorkflowRunDigest;
pub mod WorkflowSettingsChangeRow;
pub mod WorkflowStationMeta;
pub mod WorkflowTimeline;
pub mod WorkflowTimelineArcs;
pub mod WorkflowTimelineLedge;
pub mod WorkflowTimelineStation;
pub mod WorkflowTimelineTracks;
pub mod WorkflowTruncatedNotice;
pub mod workflow_run_line;