use soroban_sdk::contracterror;

/// Every failure mode the escrow can surface to callers.
///
/// Codes are stable and part of the public ABI: clients should branch on the
/// numeric value rather than the variant name.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// `init` was called on an escrow that already has an administrator.
    AlreadyInitialized = 1,
    /// A state-mutating call was made before `init`.
    NotInitialized = 2,
    /// No job is stored under the supplied id.
    JobNotFound = 3,
    /// A proof is missing for a job that should have one.
    ProofNotFound = 4,
    /// The job is not in the state required by this operation.
    InvalidStatus = 5,
    /// `max_budget` must be strictly positive.
    InvalidBudget = 6,
    /// `rate_per_second` must be strictly positive.
    InvalidRate = 7,
    /// `max_duration_secs` (or a proof duration) must be strictly positive.
    InvalidDuration = 8,
    /// The proof claims more seconds than `max_duration_secs` allows.
    DurationExceedsMax = 9,
    /// The caller is not the address the operation is restricted to.
    Unauthorized = 10,
    /// Payout was attempted while the dispute window is still open.
    DisputeWindowActive = 11,
    /// The dispute window already closed; the challenge is too late.
    DisputeWindowElapsed = 12,
    /// The queued job has not passed its `max_duration_secs` expiry yet.
    JobNotExpired = 13,
    /// A settlement computation overflowed `i128`.
    MathOverflow = 14,
}
