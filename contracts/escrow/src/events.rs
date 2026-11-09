use soroban_sdk::{contractevent, Address, BytesN};

/// Published once when the escrow is configured.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowInitialized {
    pub admin: Address,
    pub dispute_window_secs: u64,
}

/// Published when a requester opens a job and locks collateral.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobCreated {
    #[topic]
    pub job_id: u64,
    pub requester: Address,
    pub runner: Address,
    pub payment_token: Address,
    pub max_budget: i128,
    pub rate_per_second: i128,
    pub max_duration_secs: u64,
    pub dispute_window_secs: u64,
}

/// Published when the runner submits an execution proof.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofSubmitted {
    #[topic]
    pub job_id: u64,
    pub runner: Address,
    pub duration_secs: u64,
    pub exit_code: i32,
    pub output_hash: BytesN<32>,
}

/// Published when a completed job settles and the escrow drains.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutClaimed {
    #[topic]
    pub job_id: u64,
    pub requester: Address,
    pub runner: Address,
    pub earnings: i128,
    pub refund: i128,
}

/// Published when the requester challenges a proof inside the dispute window.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobDisputed {
    #[topic]
    pub job_id: u64,
    pub requester: Address,
}

/// Published when a queued job expires and the requester is refunded in full.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobRefunded {
    #[topic]
    pub job_id: u64,
    pub requester: Address,
    pub amount: i128,
}

/// Published when the admin resolves a disputed job and splits the escrow.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolved {
    #[topic]
    pub job_id: u64,
    pub admin: Address,
    pub runner_award: i128,
    pub requester_refund: i128,
}

/// Published when the admin changes the dispute window.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeWindowUpdated {
    pub admin: Address,
    pub dispute_window_secs: u64,
}
