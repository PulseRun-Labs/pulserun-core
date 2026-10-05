#![no_std]
#![allow(clippy::too_many_arguments)]

//! # PulseRun Escrow
//!
//! On-chain escrow and settlement for PulseRun, a pay-per-run compute and CI
//! runner protocol on Stellar. A requester locks a budget up front, a runner
//! executes the job and submits a metered proof, and settlement pays the runner
//! for the seconds actually executed while returning the unspent remainder.
//!
//! ## Guarantees
//!
//! * **Collateral is locked before work starts.** `create_job` moves
//!   `max_budget` into the contract, so a runner is always funded.
//! * **Runners cannot overbill.** A proof whose `duration_secs` exceeds
//!   `max_duration_secs` is rejected, and payout is hard-capped at `max_budget`.
//! * **Requesters get a challenge window.** Payout is blocked until
//!   `completed_at + dispute_window_secs`; `dispute_job` halts it entirely.
//! * **Stuck jobs are recoverable.** A job that is still `Queued` after
//!   `max_duration_secs` can be cancelled for a full refund.

use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, BytesN, Env};

mod errors;
mod storage;
mod types;

#[cfg(test)]
mod test;

pub use crate::errors::Error;
pub use crate::types::{ComputeJob, ExecutionProof, JobStatus};

/// The PulseRun escrow contract.
#[contract]
pub struct PulseEscrow;

#[contractimpl]
impl PulseEscrow {
    /// One-time configuration.
    ///
    /// * `admin` – administrator address; must authorize the call.
    /// * `dispute_window_secs` – seconds a requester has to challenge a proof
    ///   after it lands, before the runner may claim.
    pub fn init(env: Env, admin: Address, dispute_window_secs: u64) -> Result<(), Error> {
        admin.require_auth();

        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        storage::set_admin(&env, &admin);
        storage::set_dispute_window(&env, dispute_window_secs);
        Ok(())
    }

    /// Opens an escrow and locks the requester's collateral.
    ///
    /// Transfers `max_budget` of `payment_token` from `requester` into the
    /// contract and records a [`ComputeJob`] in the `Queued` state. Returns the
    /// newly allocated `job_id`.
    pub fn create_job(
        env: Env,
        requester: Address,
        runner: Address,
        payment_token: Address,
        max_budget: i128,
        rate_per_second: i128,
        max_duration_secs: u64,
    ) -> Result<u64, Error> {
        storage::require_initialized(&env)?;
        requester.require_auth();

        if max_budget <= 0 {
            return Err(Error::InvalidBudget);
        }
        if rate_per_second <= 0 {
            return Err(Error::InvalidRate);
        }
        if max_duration_secs == 0 {
            return Err(Error::InvalidDuration);
        }

        // Lock collateral before any state is written: if the transfer fails the
        // whole invocation reverts, so a job can never exist unfunded.
        TokenClient::new(&env, &payment_token).transfer(
            &requester,
            env.current_contract_address(),
            &max_budget,
        );

        let job_id = storage::next_job_id(&env);
        let job = ComputeJob {
            job_id,
            requester,
            runner,
            payment_token,
            max_budget,
            rate_per_second,
            max_duration_secs,
            status: JobStatus::Queued,
            created_at: env.ledger().timestamp(),
            completed_at: 0,
            output_hash: BytesN::from_array(&env, &[0u8; 32]),
        };
        storage::set_job(&env, &job);

        Ok(job_id)
    }

    /// Accepts a runner's [`ExecutionProof`] and opens the dispute window.
    ///
    /// The caller must be the job's designated runner. `duration_secs` is
    /// validated against `max_duration_secs` before the output hash is locked
    /// in and the dispute clock starts.
    pub fn submit_proof(env: Env, runner: Address, proof: ExecutionProof) -> Result<(), Error> {
        storage::require_initialized(&env)?;

        let mut job = storage::get_job(&env, proof.job_id).ok_or(Error::JobNotFound)?;

        if job.status != JobStatus::Queued {
            return Err(Error::InvalidStatus);
        }
        if runner != job.runner {
            return Err(Error::Unauthorized);
        }
        runner.require_auth();

        if proof.duration_secs == 0 {
            return Err(Error::InvalidDuration);
        }
        if proof.duration_secs > job.max_duration_secs {
            return Err(Error::DurationExceedsMax);
        }

        job.status = JobStatus::Completed;
        job.completed_at = env.ledger().timestamp();
        job.output_hash = proof.output_hash.clone();

        storage::set_job(&env, &job);
        storage::set_proof(&env, &proof);

        Ok(())
    }

    /// Settles a completed job once the dispute window has elapsed.
    ///
    /// Computes the linear payment `rate_per_second * duration_secs`, capped at
    /// `max_budget`; pays the earnings to the runner, refunds the unspent dust
    /// to the requester, and marks the job `Settled`. Returns the payout.
    pub fn claim_payout(env: Env, job_id: u64) -> Result<i128, Error> {
        storage::require_initialized(&env)?;

        let mut job = storage::get_job(&env, job_id).ok_or(Error::JobNotFound)?;
        if job.status != JobStatus::Completed {
            return Err(Error::InvalidStatus);
        }

        let proof = storage::get_proof(&env, job_id).ok_or(Error::ProofNotFound)?;

        let release_at = job
            .completed_at
            .checked_add(storage::get_dispute_window(&env))
            .ok_or(Error::MathOverflow)?;
        if env.ledger().timestamp() < release_at {
            return Err(Error::DisputeWindowActive);
        }

        let earnings = linear_payment(job.rate_per_second, proof.duration_secs, job.max_budget)?;
        let refund = job.max_budget - earnings;

        // Mark settled *before* external calls so a re-entrant token cannot
        // observe the job as still claimable.
        job.status = JobStatus::Settled;
        storage::set_job(&env, &job);

        let token = TokenClient::new(&env, &job.payment_token);
        let contract = env.current_contract_address();
        if earnings > 0 {
            token.transfer(&contract, &job.runner, &earnings);
        }
        if refund > 0 {
            token.transfer(&contract, &job.requester, &refund);
        }

        Ok(earnings)
    }

    /// Challenges a completed proof inside the dispute window.
    ///
    /// Only the requester may dispute, and only before
    /// `completed_at + dispute_window_secs` passes. On success the job moves to
    /// `Disputed` and `claim_payout` will refuse it, halting automatic payout
    /// pending off-chain resolution.
    pub fn dispute_job(env: Env, requester: Address, job_id: u64) -> Result<(), Error> {
        storage::require_initialized(&env)?;

        let mut job = storage::get_job(&env, job_id).ok_or(Error::JobNotFound)?;
        if job.status != JobStatus::Completed {
            return Err(Error::InvalidStatus);
        }
        if requester != job.requester {
            return Err(Error::Unauthorized);
        }
        requester.require_auth();

        let deadline = job
            .completed_at
            .checked_add(storage::get_dispute_window(&env))
            .ok_or(Error::MathOverflow)?;
        if env.ledger().timestamp() >= deadline {
            return Err(Error::DisputeWindowElapsed);
        }

        job.status = JobStatus::Disputed;
        storage::set_job(&env, &job);
        Ok(())
    }

    /// Refunds a job that was never executed.
    ///
    /// A job still in `Queued` may be cancelled by its requester once
    /// `max_duration_secs` has elapsed since `created_at`, since the runner had
    /// its full execution budget and produced nothing. The entire locked
    /// `max_budget` is returned and the job is marked `Refunded`.
    pub fn cancel_unclaimed_job(env: Env, requester: Address, job_id: u64) -> Result<(), Error> {
        storage::require_initialized(&env)?;

        let mut job = storage::get_job(&env, job_id).ok_or(Error::JobNotFound)?;
        if job.status != JobStatus::Queued {
            return Err(Error::InvalidStatus);
        }
        if requester != job.requester {
            return Err(Error::Unauthorized);
        }
        requester.require_auth();

        let expiry = job
            .created_at
            .checked_add(job.max_duration_secs)
            .ok_or(Error::MathOverflow)?;
        if env.ledger().timestamp() < expiry {
            return Err(Error::JobNotExpired);
        }

        job.status = JobStatus::Refunded;
        storage::set_job(&env, &job);

        TokenClient::new(&env, &job.payment_token).transfer(
            &env.current_contract_address(),
            &job.requester,
            &job.max_budget,
        );

        Ok(())
    }

    // ------------------------------------------------------------------
    // Read-only views
    // ------------------------------------------------------------------

    /// Returns the stored job, or [`Error::JobNotFound`].
    pub fn get_job(env: Env, job_id: u64) -> Result<ComputeJob, Error> {
        storage::get_job(&env, job_id).ok_or(Error::JobNotFound)
    }

    /// Returns the stored proof, or [`Error::ProofNotFound`].
    pub fn get_proof(env: Env, job_id: u64) -> Result<ExecutionProof, Error> {
        storage::get_proof(&env, job_id).ok_or(Error::ProofNotFound)
    }

    /// Number of jobs ever created.
    pub fn job_count(env: Env) -> u64 {
        storage::job_count(&env)
    }

    /// Configured dispute window, in seconds.
    pub fn dispute_window(env: Env) -> u64 {
        storage::get_dispute_window(&env)
    }

    /// Configured administrator.
    pub fn admin(env: Env) -> Address {
        storage::get_admin(&env)
    }
}

/// Linear metered price, clamped to the escrow ceiling.
fn linear_payment(
    rate_per_second: i128,
    duration_secs: u64,
    max_budget: i128,
) -> Result<i128, Error> {
    let metered = rate_per_second
        .checked_mul(duration_secs as i128)
        .ok_or(Error::MathOverflow)?;
    Ok(if metered > max_budget {
        max_budget
    } else {
        metered
    })
}
