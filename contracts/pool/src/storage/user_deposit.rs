use proc_macros::{data_storage_type, extend_ttl_info, SorobanData};
use shared::consts::DAY_IN_LEDGERS;
use shared::soroban_data::SorobanData;
use soroban_sdk::{contracttype, Address, Env};

use crate::storage::data_key::DataKey;

use super::double_value::DoubleU128;

const BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const LIFETIME_THRESHOLD: u32 = BUMP_AMOUNT - DAY_IN_LEDGERS;

#[contracttype]
#[derive(Clone, Debug, SorobanData, Default)]
#[data_storage_type(Persistent)]
#[extend_ttl_info(BUMP_AMOUNT, LIFETIME_THRESHOLD)]
pub struct UserDeposit {
    pub lp_amount: u128,
    pub reward_debts: DoubleU128,
}

impl UserDeposit {
    pub fn get(env: &Env, address: Address) -> UserDeposit {
        UserDeposit::get_by_key(env, &DataKey::UserDeposit(address)).unwrap_or_default()
    }

    pub fn save(&self, env: &Env, address: Address) {
        self.save_by_key(env, &DataKey::UserDeposit(address));
    }
}
