use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{MockToken, MockTokenClient, TokenError};

fn setup(env: &Env) -> (MockTokenClient<'_>, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register(MockToken, ());
    let client = MockTokenClient::new(env, &contract_id);
    let alice = Address::generate(env);
    let bob = Address::generate(env);
    (client, alice, bob)
}

#[test]
fn mint_sets_balance_from_zero() {
    let env = Env::default();
    let (client, alice, _bob) = setup(&env);

    client.mint(&alice, &1_000);
    assert_eq!(client.balance(&alice), 1_000);
}

#[test]
fn transfer_moves_funds_and_requires_auth() {
    let env = Env::default();
    let (client, alice, bob) = setup(&env);

    client.mint(&alice, &500);
    client.transfer(&alice, &bob, &200);

    assert_eq!(client.balance(&alice), 300);
    assert_eq!(client.balance(&bob), 200);
}

#[test]
fn transfer_rejects_insufficient_balance() {
    let env = Env::default();
    let (client, alice, bob) = setup(&env);

    client.mint(&alice, &10);
    assert_eq!(
        client.try_transfer(&alice, &bob, &11),
        Err(Ok(TokenError::InsufficientBalance))
    );
}

#[test]
fn transfer_rejects_non_positive_amount() {
    let env = Env::default();
    let (client, alice, bob) = setup(&env);

    client.mint(&alice, &10);
    assert_eq!(
        client.try_transfer(&alice, &bob, &0),
        Err(Ok(TokenError::InvalidAmount))
    );
}

#[test]
fn mint_rejects_non_positive_amount() {
    let env = Env::default();
    let (client, alice, _bob) = setup(&env);

    assert_eq!(
        client.try_mint(&alice, &0),
        Err(Ok(TokenError::InvalidAmount))
    );
}

#[test]
fn burn_reduces_balance() {
    let env = Env::default();
    let (client, alice, _bob) = setup(&env);

    client.mint(&alice, &100);
    client.burn(&alice, &40);
    assert_eq!(client.balance(&alice), 60);
}
