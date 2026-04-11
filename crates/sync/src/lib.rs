//! Agentora CRDT状态同步
//!
//! 自实现LWW-Register、G-Counter、OR-Set、操作签名验证、Merkle校验。

pub mod types;
pub mod codec;
pub mod lww;
pub mod gcounter;
pub mod orset;
pub mod signature;
pub mod merkle;
pub mod state;

pub use types::PeerId;
pub use codec::CrdtOp;
pub use lww::LwwRegister;
pub use gcounter::GCounter;
pub use orset::OrSet;
pub use state::SyncState;