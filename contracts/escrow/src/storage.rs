use soroban_sdk::{contracttype, Address, Env};

use crate::errors::Error;
use crate::types::{ComputeJob, ExecutionProof};

/// Number of ledgers used as the TTL bump threshold (~1 day at 5s/ledger).
const TTL_THRESHOLD: u32 = 17_280;
/// TTL the persistent entries are extended to (~30 days at 5s/ledger).
const TTL_EXTEND_TO: u32 = 518_400;

/// All storage keys used by the escrow.
///
/// `Admin`, `DisputeWindow` and `JobCount` are contract-wide configuration and
/// live in instance storage; per-job records live in persistent storage so a
/// large backlog does not blow the instance footprint.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    DisputeWindow,
    JobCount,
    Job(u64),
    Proof(u64),
}

/// Returns `true` once `init` has stored an administrator.
pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

/// Fails with [`Error::NotInitialized`] unless `init` has run.
pub fn require_initialized(env: &Env) -> Result<(), Error> {
    if has_admin(env) {
        Ok(())
    } else {
        Err(Error::NotInitialized)
    }
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .expect("escrow is not initialized")
}

pub fn set_dispute_window(env: &Env, secs: u64) {
    env.storage().instance().set(&DataKey::DisputeWindow, &secs);
}

pub fn get_dispute_window(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::DisputeWindow)
        .unwrap_or(0)
}

/// Allocates the next job id (1, 2, 3, ...) and persists the counter.
pub fn next_job_id(env: &Env) -> u64 {
    let count: u64 = env
        .storage()
        .instance()
        .get(&DataKey::JobCount)
        .unwrap_or(0);
    let job_id = count + 1;
    env.storage().instance().set(&DataKey::JobCount, &job_id);
    job_id
}

/// Read-only accessor for the number of jobs ever created.
pub fn job_count(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::JobCount)
        .unwrap_or(0)
}

pub fn set_job(env: &Env, job: &ComputeJob) {
    let key = DataKey::Job(job.job_id);
    env.storage().persistent().set(&key, job);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

pub fn get_job(env: &Env, job_id: u64) -> Option<ComputeJob> {
    env.storage().persistent().get(&DataKey::Job(job_id))
}

pub fn set_proof(env: &Env, proof: &ExecutionProof) {
    let key = DataKey::Proof(proof.job_id);
    env.storage().persistent().set(&key, proof);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

pub fn get_proof(env: &Env, job_id: u64) -> Option<ExecutionProof> {
    env.storage().persistent().get(&DataKey::Proof(job_id))
}
