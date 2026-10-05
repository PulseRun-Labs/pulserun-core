use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env,
};

use mock_token::{MockToken, MockTokenClient};

use crate::{Error, ExecutionProof, JobStatus, PulseEscrow, PulseEscrowClient};

const DISPUTE_WINDOW: u64 = 300;
const FUNDING: i128 = 100_000;

/// Everything a test needs to drive the escrow, by value so clients can be
/// constructed on a per-test basis.
struct World {
    env: Env,
    escrow_id: Address,
    token_id: Address,
    admin: Address,
    requester: Address,
    runner: Address,
}

impl World {
    fn escrow(&self) -> PulseEscrowClient<'_> {
        PulseEscrowClient::new(&self.env, &self.escrow_id)
    }

    fn token(&self) -> MockTokenClient<'_> {
        MockTokenClient::new(&self.env, &self.token_id)
    }
}

/// Boots a fresh environment: a registered mock token funded for the requester,
/// a registered escrow initialized with [`DISPUTE_WINDOW`].
fn setup() -> World {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let requester = Address::generate(&env);
    let runner = Address::generate(&env);

    let token_id = env.register(MockToken, ());
    MockTokenClient::new(&env, &token_id).mint(&requester, &FUNDING);

    let escrow_id = env.register(PulseEscrow, ());
    PulseEscrowClient::new(&env, &escrow_id).init(&admin, &DISPUTE_WINDOW);

    World {
        env,
        escrow_id,
        token_id,
        admin,
        requester,
        runner,
    }
}

fn hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn proof(env: &Env, job_id: u64, duration_secs: u64, exit_code: i32) -> ExecutionProof {
    ExecutionProof {
        job_id,
        duration_secs,
        exit_code,
        output_hash: hash(env, 7),
    }
}

// ---------------------------------------------------------------------------
// init
// ---------------------------------------------------------------------------

#[test]
fn init_records_admin_and_dispute_window() {
    let w = setup();
    let escrow = w.escrow();

    assert_eq!(escrow.admin(), w.admin);
    assert_eq!(escrow.dispute_window(), DISPUTE_WINDOW);
    assert_eq!(escrow.job_count(), 0);
}

#[test]
fn init_twice_is_rejected() {
    let w = setup();
    let escrow = w.escrow();

    assert_eq!(
        escrow.try_init(&w.admin, &DISPUTE_WINDOW),
        Err(Ok(Error::AlreadyInitialized))
    );
}

// ---------------------------------------------------------------------------
// create_job
// ---------------------------------------------------------------------------

#[test]
fn create_job_locks_collateral_and_allocates_ids() {
    let w = setup();
    let escrow = w.escrow();
    let token = w.token();

    let first = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
    let second = escrow.create_job(&w.requester, &w.runner, &w.token_id, &2_000, &20, &200);

    assert_eq!(first, 1);
    assert_eq!(second, 2);
    assert_eq!(escrow.job_count(), 2);

    // Collateral is now custodied by the contract.
    assert_eq!(token.balance(&w.escrow_id), 3_000);
    assert_eq!(token.balance(&w.requester), FUNDING - 3_000);

    let job = escrow.get_job(&first);
    assert_eq!(job.status, JobStatus::Queued);
    assert_eq!(job.max_budget, 1_000);
    assert_eq!(job.rate_per_second, 10);
    assert_eq!(job.max_duration_secs, 100);
    assert_eq!(job.output_hash, hash(&w.env, 0));
}

#[test]
fn create_job_rejects_invalid_parameters() {
    let w = setup();
    let escrow = w.escrow();

    assert_eq!(
        escrow.try_create_job(&w.requester, &w.runner, &w.token_id, &0, &10, &100),
        Err(Ok(Error::InvalidBudget))
    );
    assert_eq!(
        escrow.try_create_job(&w.requester, &w.runner, &w.token_id, &1_000, &0, &100),
        Err(Ok(Error::InvalidRate))
    );
    assert_eq!(
        escrow.try_create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &0),
        Err(Ok(Error::InvalidDuration))
    );
}

#[test]
fn create_job_before_init_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let escrow_id = env.register(PulseEscrow, ());
    let escrow = PulseEscrowClient::new(&env, &escrow_id);

    let requester = Address::generate(&env);
    let runner = Address::generate(&env);
    let token_id = env.register(MockToken, ());
    MockTokenClient::new(&env, &token_id).mint(&requester, &FUNDING);

    assert_eq!(
        escrow.try_create_job(&requester, &runner, &token_id, &1_000, &10, &100),
        Err(Ok(Error::NotInitialized))
    );
}

#[test]
fn create_job_requires_requester_authorization() {
    // Deliberately never call `mock_all_auths`: `create_job` must fail without a
    // real requester signature. The escrow is configured by writing storage
    // directly rather than via `init`, which itself needs admin authorization.
    let env = Env::default();
    let admin = Address::generate(&env);
    let requester = Address::generate(&env);
    let runner = Address::generate(&env);

    let token_id = env.register(MockToken, ());
    MockTokenClient::new(&env, &token_id).mint(&requester, &FUNDING);

    let escrow_id = env.register(PulseEscrow, ());
    env.as_contract(&escrow_id, || {
        crate::storage::set_admin(&env, &admin);
        crate::storage::set_dispute_window(&env, DISPUTE_WINDOW);
    });
    let escrow = PulseEscrowClient::new(&env, &escrow_id);

    assert!(escrow
        .try_create_job(&requester, &runner, &token_id, &1_000, &10, &100)
        .is_err());
}

// ---------------------------------------------------------------------------
// submit_proof
// ---------------------------------------------------------------------------

#[test]
fn submit_proof_locks_output_hash_and_opens_window() {
    let w = setup();
    let escrow = w.escrow();

    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
    w.env.ledger().set_timestamp(5_000);
    escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 40, 0));

    let job = escrow.get_job(&job_id);
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.completed_at, 5_000);
    assert_eq!(job.output_hash, hash(&w.env, 7));

    let stored = escrow.get_proof(&job_id);
    assert_eq!(stored.duration_secs, 40);
    assert_eq!(stored.exit_code, 0);
}

#[test]
fn submit_proof_rejects_non_runner() {
    let w = setup();
    let escrow = w.escrow();
    let impostor = Address::generate(&w.env);

    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);

    assert_eq!(
        escrow.try_submit_proof(&impostor, &proof(&w.env, job_id, 10, 0)),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn submit_proof_enforces_duration_bounds() {
    let w = setup();
    let escrow = w.escrow();

    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);

    // Zero-length executions are meaningless.
    assert_eq!(
        escrow.try_submit_proof(&w.runner, &proof(&w.env, job_id, 0, 0)),
        Err(Ok(Error::InvalidDuration))
    );
    // A runner may not bill past the agreed ceiling.
    assert_eq!(
        escrow.try_submit_proof(&w.runner, &proof(&w.env, job_id, 101, 0)),
        Err(Ok(Error::DurationExceedsMax))
    );
    // A duration exactly at the ceiling is accepted.
    escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 100, 1));
    assert_eq!(w.escrow().get_job(&job_id).status, JobStatus::Completed);
}

#[test]
fn submit_proof_is_single_use() {
    let w = setup();
    let escrow = w.escrow();

    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
    escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 10, 0));

    assert_eq!(
        escrow.try_submit_proof(&w.runner, &proof(&w.env, job_id, 10, 0)),
        Err(Ok(Error::InvalidStatus))
    );
}

#[test]
fn submit_proof_for_unknown_job_is_rejected() {
    let w = setup();
    assert_eq!(
        w.escrow()
            .try_submit_proof(&w.runner, &proof(&w.env, 42, 10, 0)),
        Err(Ok(Error::JobNotFound))
    );
}

// ---------------------------------------------------------------------------
// claim_payout / happy path
// ---------------------------------------------------------------------------

#[test]
fn happy_path_pays_runner_and_refunds_unspent_dust() {
    let w = setup();
    let escrow = w.escrow();
    let token = w.token();

    w.env.ledger().set_timestamp(1_000);
    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);

    w.env.ledger().set_timestamp(1_020);
    escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 20, 0));

    // Exactly at the window boundary payout is still blocked...
    w.env.ledger().set_timestamp(1_020 + DISPUTE_WINDOW - 1);
    assert_eq!(
        escrow.try_claim_payout(&job_id),
        Err(Ok(Error::DisputeWindowActive))
    );

    // ...and opens once the window elapses.
    w.env.ledger().set_timestamp(1_020 + DISPUTE_WINDOW);
    let payout = escrow.claim_payout(&job_id);

    assert_eq!(payout, 200); // 10 base units/s * 20s
    assert_eq!(token.balance(&w.runner), 200);
    assert_eq!(token.balance(&w.requester), FUNDING - 200); // 800 dust refunded
    assert_eq!(token.balance(&w.escrow_id), 0); // escrow fully drained

    assert_eq!(escrow.get_job(&job_id).status, JobStatus::Settled);
}

#[test]
fn payout_is_capped_at_max_budget() {
    let w = setup();
    let escrow = w.escrow();
    let token = w.token();

    // rate * duration = 100 * 100 = 10_000, far above the 1_000 budget.
    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &100, &100);
    escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 100, 0));
    w.env.ledger().set_timestamp(DISPUTE_WINDOW);

    let payout = escrow.claim_payout(&job_id);

    assert_eq!(payout, 1_000);
    assert_eq!(token.balance(&w.runner), 1_000);
    assert_eq!(token.balance(&w.requester), FUNDING - 1_000);
    assert_eq!(token.balance(&w.escrow_id), 0);
}

#[test]
fn claim_is_blocked_on_queued_and_unknown_jobs() {
    let w = setup();
    let escrow = w.escrow();

    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
    assert_eq!(
        escrow.try_claim_payout(&job_id),
        Err(Ok(Error::InvalidStatus))
    );
    assert_eq!(escrow.try_claim_payout(&999), Err(Ok(Error::JobNotFound)));
}

// ---------------------------------------------------------------------------
// disputes
// ---------------------------------------------------------------------------

#[test]
fn dispute_halts_automatic_payout() {
    let w = setup();
    let escrow = w.escrow();
    let token = w.token();

    w.env.ledger().set_timestamp(1_000);
    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
    escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 20, 0));

    // Challenge mid-window.
    w.env.ledger().set_timestamp(1_050);
    escrow.dispute_job(&w.requester, &job_id);
    assert_eq!(escrow.get_job(&job_id).status, JobStatus::Disputed);

    // Even long after the window, a disputed job will not auto-settle.
    w.env
        .ledger()
        .set_timestamp(1_020 + DISPUTE_WINDOW + 10_000);
    assert_eq!(
        escrow.try_claim_payout(&job_id),
        Err(Ok(Error::InvalidStatus))
    );

    // Funds stay escrowed pending off-chain resolution.
    assert_eq!(token.balance(&w.escrow_id), 1_000);
    assert_eq!(token.balance(&w.runner), 0);
}

#[test]
fn dispute_after_window_is_rejected() {
    let w = setup();
    let escrow = w.escrow();

    w.env.ledger().set_timestamp(1_000);
    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
    escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 20, 0));

    w.env.ledger().set_timestamp(1_020 + DISPUTE_WINDOW);
    assert_eq!(
        escrow.try_dispute_job(&w.requester, &job_id),
        Err(Ok(Error::DisputeWindowElapsed))
    );
}

#[test]
fn only_requester_may_dispute() {
    let w = setup();
    let escrow = w.escrow();
    let outsider = Address::generate(&w.env);

    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
    escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 20, 0));

    assert_eq!(
        escrow.try_dispute_job(&outsider, &job_id),
        Err(Ok(Error::Unauthorized))
    );
}

// ---------------------------------------------------------------------------
// cancel_unclaimed_job / timeout refunds
// ---------------------------------------------------------------------------

#[test]
fn cancel_unclaimed_job_refunds_full_budget_after_expiry() {
    let w = setup();
    let escrow = w.escrow();
    let token = w.token();

    w.env.ledger().set_timestamp(1_000);
    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
    assert_eq!(token.balance(&w.escrow_id), 1_000);

    // One second before the runner's full execution budget has elapsed the job
    // is still claimable by the runner, so cancellation is refused.
    w.env.ledger().set_timestamp(1_000 + 100 - 1);
    assert_eq!(
        escrow.try_cancel_unclaimed_job(&w.requester, &job_id),
        Err(Ok(Error::JobNotExpired))
    );

    // At expiry the requester is made whole.
    w.env.ledger().set_timestamp(1_000 + 100);
    escrow.cancel_unclaimed_job(&w.requester, &job_id);

    assert_eq!(escrow.get_job(&job_id).status, JobStatus::Refunded);
    assert_eq!(token.balance(&w.requester), FUNDING);
    assert_eq!(token.balance(&w.escrow_id), 0);
}

#[test]
fn cancel_unclaimed_job_rejects_completed_jobs() {
    let w = setup();
    let escrow = w.escrow();

    w.env.ledger().set_timestamp(1_000);
    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);
    escrow.submit_proof(&w.runner, &proof(&w.env, job_id, 20, 0));

    w.env.ledger().set_timestamp(1_000 + 100 + 1);
    assert_eq!(
        escrow.try_cancel_unclaimed_job(&w.requester, &job_id),
        Err(Ok(Error::InvalidStatus))
    );
}

#[test]
fn only_requester_may_cancel() {
    let w = setup();
    let escrow = w.escrow();
    let outsider = Address::generate(&w.env);

    let job_id = escrow.create_job(&w.requester, &w.runner, &w.token_id, &1_000, &10, &100);

    assert_eq!(
        escrow.try_cancel_unclaimed_job(&outsider, &job_id),
        Err(Ok(Error::Unauthorized))
    );
}

// ---------------------------------------------------------------------------
// views
// ---------------------------------------------------------------------------

#[test]
fn unknown_job_and_proof_views_fail() {
    let w = setup();
    let escrow = w.escrow();

    assert_eq!(escrow.try_get_job(&1), Err(Ok(Error::JobNotFound)));
    assert_eq!(escrow.try_get_proof(&1), Err(Ok(Error::ProofNotFound)));
}
