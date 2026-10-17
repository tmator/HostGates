//! Contracts for phases 2–6; no simulated connection implementations.
use crate::{credentials::Secret, error::Result, models::Session};
use std::{future::Future, path::PathBuf, pin::Pin};
pub type Task<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
pub struct ConnectionId(pub String);
pub enum Authentication {
    Password(Secret),
    PrivateKey {
        path: PathBuf,
        passphrase: Option<Secret>,
    },
    Agent,
    KeyboardInteractive,
}
pub struct HostKeyChallenge {
    pub hostname: String,
    pub port: u16,
    pub ip: Option<String>,
    pub algorithm: String,
    pub fingerprint: String,
    pub changed: bool,
}
pub enum TrustDecision {
    Once,
    Always,
    Reject,
}
pub trait HostKeyVerifier: Send + Sync {
    fn verify<'a>(&'a self, key: HostKeyChallenge) -> Task<'a, TrustDecision>;
}
pub trait SshBackend: Send + Sync {
    fn connect<'a>(
        &'a self,
        session: &'a Session,
        auth: Authentication,
        verifier: &'a dyn HostKeyVerifier,
    ) -> Task<'a, ConnectionId>;
    fn write<'a>(&'a self, id: &'a ConnectionId, bytes: Vec<u8>) -> Task<'a, ()>;
    fn read<'a>(&'a self, id: &'a ConnectionId) -> Task<'a, Vec<u8>>;
    fn resize<'a>(&'a self, id: &'a ConnectionId, cols: u16, rows: u16) -> Task<'a, ()>;
    fn disconnect<'a>(&'a self, id: &'a ConnectionId) -> Task<'a, ()>;
}
pub struct RdpProcess {
    pub pid: u32,
}
pub trait RdpBackend: Send + Sync {
    fn available(&self) -> bool;
    fn launch<'a>(&'a self, session: &'a Session) -> Task<'a, RdpProcess>;
    fn is_running(&self, process: &RdpProcess) -> Result<bool>;
    fn close(&self, process: &RdpProcess) -> Result<()>;
}
pub struct RemoteEntry {
    pub name: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: Option<u64>,
    pub permissions: Option<u32>,
}
pub trait SftpBackend: Send + Sync {
    fn list<'a>(&'a self, id: &'a ConnectionId, path: &'a str) -> Task<'a, Vec<RemoteEntry>>;
    fn upload<'a>(
        &'a self,
        id: &'a ConnectionId,
        local: PathBuf,
        remote: String,
    ) -> Task<'a, String>;
    fn download<'a>(
        &'a self,
        id: &'a ConnectionId,
        remote: String,
        local: PathBuf,
    ) -> Task<'a, String>;
    fn cancel<'a>(&'a self, transfer_id: &'a str) -> Task<'a, ()>;
}
