#![allow(dead_code)]

use core::cmp::Ordering;
use ethnum::U256;
use shared::{require, utils::num::*, Error};
use soroban_sdk::{contracttype, Address, Env};

use crate::storage::{
    claimable_balance::ClaimableBalance,
    pool::{Pool, Tokens},
    user_deposit::UserDeposit,
};

#[contracttype]
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    A2B,
    B2A,
}

impl Direction {
    pub fn to_tokens(&self) -> (Tokens, Tokens) {
        match self {
            Direction::A2B => (Tokens::TokenA, Tokens::TokenB),
            Direction::B2A => (Tokens::TokenB, Tokens::TokenA),
        }
    }
}

impl Pool {
    const MAX_TOKEN_BALANCE: u128 = 2u128.pow(40);
    const BP: u128 = 10000;

    pub const P: u128 = 48;
    const SYSTEM_PRECISION: u32 = 3;

    pub fn deposit(
        &mut self,
        env: &Env,
        amount_sp: u128,
        sender: Address,
        user: &mut UserDeposit,
    ) -> Result<(u128, u128), Error> {
        let old_d = self.d;

        require!(amount_sp > 0, Error::ZeroAmount);

        self.reserves += amount_sp;

        let old_balance = self.token_a_balance + self.token_b_balance;
        let (token_a_amount, token_b_amount) = if old_d == 0 || old_balance == 0 {
            let half_amount = amount_sp >> 1;
            self.token_a_balance = half_amount;
            self.token_b_balance = half_amount;

            (half_amount, half_amount)
        } else {
            let token_a_amount = amount_sp * self.token_a_balance / old_balance;
            let token_b_amount = amount_sp * self.token_b_balance / old_balance;
            self.token_a_balance += token_a_amount;
            self.token_b_balance += token_b_amount;

            (token_a_amount, token_b_amount)
        };

        self.update_d();

        require!(
            self.token_a_balance + self.token_b_balance < Self::MAX_TOKEN_BALANCE,
            Error::PoolOverflow
        );

        self.validate_balance_ratio()?;

        let lp_amount = self.d - old_d;

        self.get_token_a(env).transfer(
            &sender,
            &env.current_contract_address(),
            &(self.amount_from_system_precision(token_a_amount, self.decimals_a) as i128),
        );
        self.get_token_b(env).transfer(
            &sender,
            &env.current_contract_address(),
            &(self.amount_from_system_precision(token_b_amount, self.decimals_b) as i128),
        );
        self.get_lp_native_asset(env).mint(
            &sender,
            &(self.amount_from_system_precision(lp_amount, self.decimals_lp) as i128),
        );

        Ok((self.deposit_lp(user, lp_amount), lp_amount))
    }

    pub fn withdraw(
        &mut self,
        env: &Env,
        sender: Address,
        user: &mut UserDeposit,
        amount_lp: u128,
    ) -> Result<(), Error> {
        let reward_amount = self.withdraw_lp(user, amount_lp);

        let old_balance = self.token_a_balance + self.token_b_balance;
        let token_a_amount = amount_lp * self.token_a_balance / old_balance;
        let token_b_amount = amount_lp * self.token_b_balance / old_balance;

        self.token_a_balance -= token_a_amount;
        self.token_b_balance -= token_b_amount;

        require!(
            self.token_a_balance + self.token_b_balance < old_balance,
            Error::ZeroChanges
        );
        require!(amount_lp <= self.reserves, Error::ReservesExhausted);

        self.reserves -= amount_lp;
        let old_d = self.d;
        // Always equal amounts removed from actual and virtual tokens
        self.update_d();
        require!(self.d < old_d, Error::ZeroChanges);

        let token_a_amount =
            self.amount_from_system_precision(token_a_amount, self.decimals_a) + reward_amount;
        let token_b_amount =
            self.amount_from_system_precision(token_b_amount, self.decimals_b) + reward_amount;

        self.get_token_a(&env).transfer(
            &env.current_contract_address(),
            &sender,
            &(token_a_amount as i128),
        );
        self.get_token_b(&env).transfer(
            &env.current_contract_address(),
            &sender,
            &(token_b_amount as i128),
        );
        self.get_lp_token(&env).burn(
            &sender,
            &(self.amount_from_system_precision(amount_lp, self.decimals_lp) as i128),
        );

        Ok(())
    }

    pub(crate) fn deposit_lp(&mut self, user: &mut UserDeposit, lp_amount: u128) -> u128 {
        let mut pending: u128 = 0;
        if user.lp_amount > 0 {
            pending =
                ((user.lp_amount * self.acc_reward_per_share_p) >> Pool::P) - user.reward_debt;
        }
        self.total_lp_amount += lp_amount;
        user.lp_amount += lp_amount;
        user.reward_debt = (user.lp_amount * self.acc_reward_per_share_p) >> Pool::P;

        pending
    }

    pub(crate) fn withdraw_lp(&mut self, user: &mut UserDeposit, lp_amount: u128) -> u128 {
        let mut user_lp_amount: u128 = user.lp_amount;

        assert!(user_lp_amount >= lp_amount, "Not enough amount");

        let mut pending: u128 = 0;
        if user.lp_amount > 0 {
            pending =
                ((user_lp_amount * self.acc_reward_per_share_p) >> Pool::P) - user.reward_debt;
        }
        self.total_lp_amount -= lp_amount;
        user_lp_amount -= lp_amount;
        user.lp_amount = user_lp_amount;
        user.reward_debt = (user_lp_amount * self.acc_reward_per_share_p) >> Pool::P;

        pending
    }

    pub fn swap(
        &mut self,
        env: &Env,
        sender: Address,
        recipient: Address,
        amount_in: u128,
        receive_amount_min: u128,
        zero_fee: bool,
        claimable: bool,
        direction: Direction,
    ) -> Result<(u128, u128), Error> {
        let (token_from, token_to) = direction.to_tokens();
        let current_pool = env.current_contract_address();

        self.get_token_client(env, token_from).transfer(
            &current_pool,
            &sender,
            &(amount_in as i128),
        );

        let mut result = 0;
        let mut result_sp = 0;

        if amount_in == 0 {
            return Ok((0, 0));
        }

        self.set_token_balance(self.get_token_balance(token_from) + amount_in, token_from);

        let token_to_new_amount = self.get_y(self.get_token_balance(token_from));
        if self.get_token_balance(token_from) > token_to_new_amount {
            result_sp = self.get_token_balance(token_to) - token_to_new_amount;
            result = self.amount_from_system_precision(result_sp, self.decimals_a);
        }

        require!(result_sp <= self.reserves, Error::ReservesExhausted);

        // ??
        self.reserves = self.reserves + amount_in - result_sp;

        let fee = if zero_fee {
            0
        } else {
            result * self.fee_share_bp / Self::BP
        };

        result -= fee;

        self.set_token_balance(token_to_new_amount, token_to);

        self.add_rewards(fee);
        self.validate_balance_ratio()?;

        require!(
            result >= receive_amount_min,
            Error::InsufficientReceivedAmount
        );

        if claimable {
            ClaimableBalance::update(&env, recipient.clone(), |claimable_balance| {
                claimable_balance.amount += result;
                Ok(())
            })?;
        } else {
            self.get_token_client(env, token_to).transfer(
                &current_pool,
                &recipient,
                &(result as i128),
            );
        }

        Ok((result, fee))
    }

    pub fn claim_rewards(&self, user_deposit: &mut UserDeposit) -> Result<u128, Error> {
        if user_deposit.lp_amount > 0 {
            let rewards = (user_deposit.lp_amount * self.acc_reward_per_share_p) >> Pool::P;
            let pending = rewards - user_deposit.reward_debt;
            if pending > 0 {
                user_deposit.reward_debt = rewards;
            }
            return Ok(pending);
        }

        Ok(0)
    }

    pub(crate) fn add_rewards(&mut self, mut reward_amount: u128) {
        if self.total_lp_amount > 0 {
            let admin_fee_rewards = reward_amount * self.admin_fee_share_bp / Pool::BP;
            reward_amount -= admin_fee_rewards;
            self.acc_reward_per_share_p += (reward_amount << Pool::P) / self.total_lp_amount;
            self.admin_fee_amount += admin_fee_rewards;
        }
    }

    // y = (sqrt(x(4AD³ + x (4A(D - x) - D )²)) + x (4A(D - x) - D ))/8Ax
    pub fn get_y(&self, native_x: u128) -> u128 {
        let a4 = self.a << 2;
        let ddd = U256::new(self.d * self.d) * self.d;
        // 4A(D - x) - D
        let part1 = a4 as i128 * (self.d as i128 - native_x as i128) - self.d as i128;
        // x * (4AD³ + x(part1²))
        let part2 = (ddd * a4 + (U256::new((part1 * part1) as u128) * native_x)) * native_x;
        // (sqrt(part2) + x(part1)) / 8Ax)
        (sqrt(&part2).as_u128() as i128 + (native_x as i128 * part1)) as u128
            / ((self.a << 3) * native_x)
    }

    fn update_d(&mut self) {
        self.d = self.get_d(self.token_a_balance, self.token_b_balance);
    }

    pub fn get_d(&self, x: u128, y: u128) -> u128 {
        let xy: u128 = x * y;
        // Axy(x+y)
        let p1 = U256::new(self.a * (x + y) * xy);

        // xy(4A - 1) / 3
        let p2 = U256::new(xy * ((self.a << 2) - 1) / 3);

        // sqrt(p1² + p2³)
        let p3 = sqrt(&((p1 * p1) + (p2 * p2 * p2)));

        // cbrt(p1 + p3) + cbrt(p1 - p3)
        let mut d = cbrt(&(p1 + p3));
        if p3.gt(&p1) {
            d -= cbrt(&(p3 - p1));
        } else {
            d += cbrt(&(p1 - p3));
        }
        d << 1
    }

    pub(crate) fn amount_to_system_precision(&self, amount: u128, decimals: u32) -> u128 {
        match decimals.cmp(&Self::SYSTEM_PRECISION) {
            Ordering::Greater => amount / (10u128.pow(decimals - Self::SYSTEM_PRECISION)),
            Ordering::Less => amount * (10u128.pow(Self::SYSTEM_PRECISION - decimals)),
            Ordering::Equal => amount,
        }
    }

    pub(crate) fn amount_from_system_precision(&self, amount: u128, decimals: u32) -> u128 {
        match decimals.cmp(&Self::SYSTEM_PRECISION) {
            Ordering::Greater => amount * (10u128.pow(decimals - Self::SYSTEM_PRECISION)),
            Ordering::Less => amount / (10u128.pow(Self::SYSTEM_PRECISION - decimals)),
            Ordering::Equal => amount,
        }
    }

    fn validate_balance_ratio(&self) -> Result<(), Error> {
        let min = self.token_a_balance.min(self.token_b_balance);
        let max = self.token_a_balance.max(self.token_b_balance);
        require!(
            min * Self::BP / max >= self.balance_ratio_min_bp,
            Error::BalanceRatioExceeded
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use soroban_sdk::{testutils::Address as _, Address, Env};

    use crate::storage::pool::Pool;

    #[test]
    fn check_d() {
        let env = Env::default();
        let pool = Pool::from_init_params(
            20,
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
            100,
            1,
            2000,
            7,
            7,
            7,
        );

        assert_eq!(pool.get_d(0, 0), 0);
        assert_eq!(pool.get_d(100_000, 100_000), 200_000);
        assert_eq!(pool.get_d(15_819, 189_999), 200_000);
        assert_eq!(pool.get_d(295_237, 14_763), 295_240);
        assert_eq!(pool.get_d(23_504, 282_313), 297_172);
        assert_eq!(pool.get_d(104_762, 5_239), 104_764);
        assert_eq!(pool.get_d(8_133, 97_685), 102_826);
        assert_eq!(pool.get_d(4_777, 4_749), 9_526);
        assert_eq!(pool.get_d(22_221, 21_607), 43_828);

        assert!(pool.get_d(11_000_001_000, 251_819).abs_diff(2_000_000_000) <= 1_000);
        assert!(
            pool.get_d(100_118_986, 1_999_748_181)
                .abs_diff(2_000_000_000)
                <= 100
        );
    }
}
