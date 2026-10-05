use soroban_sdk::{contracttype, Address, BytesN};

/// Lifecycle of a compute job escrow.
///
/// ```text
///            create_job            submit_proof
///  (none) ──────────────► Queued ──────────────► Completed
///                           │                        │
///                           │ cancel_unclaimed_job   ├── claim_payout ──► Settled
///                           ▼ (after max_duration)   │
///                        Refunded                    └── dispute_job ──► Disputed
/// ```
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobStatus {
    /// Collateral is locked and the job is awaiting the runner's proof.
    Queued,
    /// The runner submitted a proof; the dispute window is now open.
    Completed,
    /// The requester challenged the proof; automatic payout is halted.
    Disputed,
    /// Earnings were paid to the runner and unspent dust refunded.
    Settled,
    /// The job never completed in time and the requester was made whole.
    Refunded,
}

impl JobStatus {
    /// Returns `true` once the job reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Settled | JobStatus::Refunded)
    }
}

/// The escrow record for a single pay-per-run compute job.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeJob {
    /// Monotonic identifier assigned by `create_job` (starts at 1).
    pub job_id: u64,
    /// The party paying for the compute.
    pub requester: Address,
    /// The party expected to execute the job.
    pub runner: Address,
    /// Token used to denominate `max_budget` and payouts.
    pub payment_token: Address,
    /// Collateral locked at creation; the absolute spend ceiling.
    pub max_budget: i128,
    /// Linear price, in token base units, charged per executed second.
    pub rate_per_second: i128,
    /// Upper bound on billable seconds; also the unclaimed-job timeout.
    pub max_duration_secs: u64,
    /// Current lifecycle state.
    pub status: JobStatus,
    /// Ledger timestamp at which the escrow was opened.
    pub created_at: u64,
    /// Ledger timestamp at which the proof was accepted (0 while queued).
    pub completed_at: u64,
    /// Commitment to the run's output; zeroed until a proof lands.
    pub output_hash: BytesN<32>,
}

/// The runner's attestation that a job executed, with the metered duration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionProof {
    /// Job this proof settles.
    pub job_id: u64,
    /// Measured execution time in seconds (must be `1..=max_duration_secs`).
    pub duration_secs: u64,
    /// Process exit code reported by the runner.
    pub exit_code: i32,
    /// Commitment to the run's output (logs, artifacts, build cache).
    pub output_hash: BytesN<32>,
}
