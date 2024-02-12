use soroban_sdk::Env;

use crate::{
    contracts::pool::Direction,
    utils::{expect_contract_error, Snapshot, TestingEnvConfig, TestingEnvironment},
};

#[test]
fn deposit_zero_amount() {
    let env = Env::default();
    let testing_env = TestingEnvironment::default(&env);
    let TestingEnvironment {
        ref pool,
        ref alice,
        ..
    } = testing_env;

    let call_result = pool.deposit(alice, (0.0, 0.0), 0.0);
    expect_contract_error(&env, call_result, shared::Error::ZeroAmount)
}

#[test]
fn deposit_slippage() {
    let env = Env::default();
    let testing_env = TestingEnvironment::default(&env);
    let TestingEnvironment {
        ref pool,
        ref alice,
        ..
    } = testing_env;

    let call_result = pool.deposit(alice, (100.0, 0.0), 100.0);
    expect_contract_error(&env, call_result, shared::Error::Slippage)
}

#[test]
fn deposit() {
    let env = Env::default();
    let testing_env = TestingEnvironment::default(&env);
    let TestingEnvironment {
        ref pool,
        ref alice,
        ..
    } = testing_env;

    let deposits = (100.0, 50.0);
    let snapshot_before = Snapshot::take(&testing_env);
    pool.deposit(alice, deposits, 150.0).unwrap();
    let snapshot_after = Snapshot::take(&testing_env);

    snapshot_before.print_change_with(&snapshot_after, Some("Deposit: 100 yusd, 50 yaro"));

    // TODO: Expected LP diff hardcode
    let expected_lp_amount = deposits.0 + deposits.1;

    pool.invariant_total_lp_less_or_equal_d().unwrap();
    TestingEnvironment::assert_deposit_event(&env, alice, expected_lp_amount, deposits);
    TestingEnvironment::assert_claimed_reward_event(&env, alice, (0.0, 0.0));
    TestingEnvironment::assert_deposit(
        snapshot_before,
        snapshot_after,
        alice,
        deposits,
        (0.0, 0.0),
        expected_lp_amount,
    );
}

#[test]
fn deposit_disbalance() {
    let env = Env::default();
    let testing_env = TestingEnvironment::default(&env);
    let TestingEnvironment {
        ref pool,
        ref alice,
        ..
    } = testing_env;

    let deposit = (50_000_000.0, 5_000.0);
    let snapshot_before = Snapshot::take(&testing_env);
    pool.deposit(alice, deposit, 0.0).unwrap();
    let snapshot_after = Snapshot::take(&testing_env);

    // TODO: Hardcode values here
    let expected_lp_amount = pool.get_lp_amount(snapshot_before.total_lp_amount);

    // TODO: Change text below to actual amounts
    snapshot_before.print_change_with(&snapshot_after, Some("Deposit: 100 yusd, 50 yaro"));

    pool.invariant_total_lp_less_or_equal_d().unwrap();
    TestingEnvironment::assert_deposit_event(&env, alice, expected_lp_amount, deposit);
    TestingEnvironment::assert_claimed_reward_event(&env, alice, (0.0, 0.0));
    TestingEnvironment::assert_deposit(
        snapshot_before,
        snapshot_after,
        alice,
        deposit,
        (0.0, 0.0),
        expected_lp_amount,
    );
}

#[test]
fn deposit_with_overflow() {
    let env = Env::default();
    let testing_env = TestingEnvironment::default(&env);
    let TestingEnvironment {
        ref pool,
        ref alice,
        ref yaro_token,
        ref yusd_token,
        ..
    } = testing_env;

    yusd_token.airdrop_amount(alice.as_ref(), 10_000_000_000.0);
    yaro_token.airdrop_amount(alice.as_ref(), 10_000_000_000.0);

    let deposits = (600_000_000.0, 600_000_000.0);
    let call_result = pool.deposit(alice, deposits, 0.0);

    expect_contract_error(&env, call_result, shared::Error::PoolOverflow)
}

// TODO: Also add test for depositing slightly less than MAX amount
//       Swap to big disbalance 1:100 and check for overflows

#[test]
fn smallest_deposit() {
    let env = Env::default();
    let testing_env = TestingEnvironment::default(&env);
    let TestingEnvironment {
        ref pool,
        ref alice,
        ..
    } = testing_env;

    let deposits = (0.001, 0.001);
    let snapshot_before = Snapshot::take(&testing_env);
    pool.deposit(alice, deposits, 0.001 + 0.001).unwrap();
    let snapshot_after = Snapshot::take(&testing_env);

    snapshot_before.print_change_with(&snapshot_after, Some("Deposit: 0.001 yusd, 0.001 yaro"));

    // TODO: Hardcode
    let expected_lp_amount = deposits.0 + deposits.1;

    pool.invariant_total_lp_less_or_equal_d().unwrap();
    TestingEnvironment::assert_deposit_event(&env, alice, expected_lp_amount, deposits);
    TestingEnvironment::assert_claimed_reward_event(&env, alice, (0.0, 0.0));
    TestingEnvironment::assert_deposit(
        snapshot_before,
        snapshot_after,
        alice,
        deposits,
        (0.0, 0.0),
        expected_lp_amount,
    );
}

#[test]
fn deposit_only_yusd() {
    let env = Env::default();
    let testing_env = TestingEnvironment::default(&env);
    let TestingEnvironment {
        ref pool,
        ref alice,
        ..
    } = testing_env;

    let deposits = (100.0, 0.0);
    let snapshot_before = Snapshot::take(&testing_env);
    pool.deposit(alice, deposits, 99.0).unwrap();
    let snapshot_after = Snapshot::take(&testing_env);

    snapshot_before.print_change_with(&snapshot_after, Some("Deposit: 100 yusd"));

    // TODO: Hardcode
    let expected_lp_amount = deposits.0 + deposits.1;

    pool.invariant_total_lp_less_or_equal_d().unwrap();
    TestingEnvironment::assert_deposit_event(&env, alice, expected_lp_amount, deposits);
    TestingEnvironment::assert_claimed_reward_event(&env, alice, (0.0, 0.0));
    TestingEnvironment::assert_deposit(
        snapshot_before,
        snapshot_after,
        alice,
        deposits,
        (0.0, 0.0),
        expected_lp_amount,
    );
}

#[test]
fn deposit_only_yaro() {
    let env = Env::default();
    let testing_env = TestingEnvironment::default(&env);
    let TestingEnvironment {
        ref pool,
        ref alice,
        ..
    } = testing_env;

    // TODO: (0, 100) below here
    let deposits = (100.0, 0.0);
    let snapshot_before = Snapshot::take(&testing_env);
    pool.deposit(alice, deposits, 99.0).unwrap();
    let snapshot_after = Snapshot::take(&testing_env);

    snapshot_before.print_change_with(&snapshot_after, Some("Deposit: 100 yaro"));

    // TODO: Same
    let expected_lp_amount = deposits.0 + deposits.1;

    pool.invariant_total_lp_less_or_equal_d().unwrap();
    TestingEnvironment::assert_deposit_event(&env, alice, expected_lp_amount, deposits);
    TestingEnvironment::assert_claimed_reward_event(&env, alice, (0.0, 0.0));
    TestingEnvironment::assert_deposit(
        snapshot_before,
        snapshot_after,
        alice,
        deposits,
        (0.0, 0.0),
        expected_lp_amount,
    );
}

#[test]
fn deposit_twice_in_different_tokens() {
    let env = Env::default();
    let testing_env = TestingEnvironment::default(&env);
    let TestingEnvironment {
        ref pool,
        ref alice,
        ..
    } = testing_env;

    let snapshot_before = Snapshot::take(&testing_env);
    pool.deposit(alice, (100.0, 0.0), 99.0).unwrap();
    pool.deposit(alice, (0.0, 100.0), 99.0).unwrap();
    let snapshot_after = Snapshot::take(&testing_env);
    let expected_lp_amount = 200.0;

    snapshot_before.print_change_with(&snapshot_after, Some("Deposit: 100 yusd, 100 yaro"));

    pool.invariant_total_lp_less_or_equal_d().unwrap();
    TestingEnvironment::assert_deposit(
        snapshot_before,
        snapshot_after,
        alice,
        (100.0, 100.0),
        (0.0, 0.0),
        expected_lp_amount,
    );
}

#[test]
fn get_reward_after_second_deposit() {
    let env = Env::default();
    let testing_env = TestingEnvironment::create(
        &env,
        TestingEnvConfig::default()
            .with_pool_fee_share_bp(0.01) // TODO: This is not BP
            .with_yaro_admin_deposit(0.0)
            .with_yusd_admin_deposit(0.0),
    );
    let TestingEnvironment {
        ref pool,
        ref alice,
        ref bob,
        ..
    } = testing_env;

    let deposits = (2000.0, 2000.0);

    // TODO: rewarsds
    let expected_rewarsds = (1.0012199, 0.9987799);

    pool.deposit(alice, deposits, 4000.0).unwrap();
    pool.swap(alice, bob, 100.0, 98.0, Direction::A2B).unwrap();
    pool.swap(bob, alice, 100.0, 99.0, Direction::B2A).unwrap();

    let snapshot_before = Snapshot::take(&testing_env);

    // TODO: Change min LP amount to expected LP diff
    pool.deposit(alice, deposits, 0.0).unwrap();
    let snapshot_after = Snapshot::take(&testing_env);
    snapshot_before.print_change_with(&snapshot_after, None);

    // TODO: Hardcode
    let expected_lp_amount = deposits.0 + deposits.1;

    pool.invariant_total_lp_less_or_equal_d().unwrap();
    TestingEnvironment::assert_deposit_event(&env, alice, expected_lp_amount, deposits);
    TestingEnvironment::assert_claimed_reward_event(&env, alice, expected_rewarsds);
    TestingEnvironment::assert_deposit(
        snapshot_before,
        snapshot_after,
        alice,
        deposits,
        expected_rewarsds,
        expected_lp_amount,
    );
}
