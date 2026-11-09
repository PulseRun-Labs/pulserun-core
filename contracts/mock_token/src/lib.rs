#![no_std]

//! # Mock Token
//!
//! A deliberately tiny fungible token used to exercise PulseRun escrow
//! settlement on a local test network. It implements just enough of the Stellar
//! token surface for the escrow's `TokenClient` (`transfer` + `balance`) plus an
//! unauthenticated `mint` for test fixtures.
//!
//! This is **not** production money: it has no allowances, no metadata, and no
//! access control on `mint`.

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

/// Number of ledgers used as the balance TTL bump threshold (~1 day).
const TTL_THRESHOLD: u32 = 17_280;
/// TTL the persistent balance entries are extended to (~30 days).
const TTL_EXTEND_TO: u32 = 518_400;

/// Storage keys for per-account balances.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Balance(Address),
}

/// Failure modes for the mock token.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum TokenError {
    /// The account does not hold enough to cover the transfer.
    InsufficientBalance = 1,
    /// Amounts must be strictly positive.
    InvalidAmount = 2,
    /// A balance calculation overflowed `i128`.
    MathOverflow = 3,
}

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    /// Creates `amount` tokens for `to`. Unauthenticated, test-only.
    pub fn mint(env: Env, to: Address, amount: i128) -> Result<(), TokenError> {
        if amount <= 0 {
            return Err(TokenError::InvalidAmount);
        }
        let balance = Self::balance(env.clone(), to.clone());
        let updated = balance
            .checked_add(amount)
            .ok_or(TokenError::MathOverflow)?;
        write_balance(&env, &to, updated);
        Ok(())
    }

    /// Destroys `amount` tokens held by `from`. Unauthenticated, test-only.
    pub fn burn(env: Env, from: Address, amount: i128) -> Result<(), TokenError> {
        if amount <= 0 {
            return Err(TokenError::InvalidAmount);
        }

        let balance = Self::balance(env.clone(), from.clone());
        if amount > balance {
            return Err(TokenError::InsufficientBalance);
        }
        write_balance(&env, &from, balance - amount);
        Ok(())
    }

    /// Balance of `id`, defaulting to `0`.
    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(id))
            .unwrap_or(0)
    }

    /// Moves `amount` from `from` to `to`. `from` must authorize the call.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), TokenError> {
        from.require_auth();

        if amount <= 0 {
            return Err(TokenError::InvalidAmount);
        }
        if from == to {
            return Ok(());
        }

        let from_balance = Self::balance(env.clone(), from.clone());
        if from_balance < amount {
            return Err(TokenError::InsufficientBalance);
        }

        let to_balance = Self::balance(env.clone(), to.clone());
        let updated_to = to_balance
            .checked_add(amount)
            .ok_or(TokenError::MathOverflow)?;
        write_balance(&env, &from, from_balance - amount);
        write_balance(&env, &to, updated_to);
        Ok(())
    }
}

fn write_balance(env: &Env, id: &Address, amount: i128) {
    let key = DataKey::Balance(id.clone());
    env.storage().persistent().set(&key, &amount);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
}

#[cfg(test)]
mod test;
