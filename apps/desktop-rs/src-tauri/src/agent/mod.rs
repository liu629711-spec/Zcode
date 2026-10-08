//! Agent 运行时：进程命令解析 + stdio 传输 + 请求配对客户端 + 会话管理。

pub mod client;
pub mod command;
pub mod manager;
pub mod methods;
pub mod transport;
