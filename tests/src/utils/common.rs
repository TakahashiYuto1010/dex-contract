use std::{
    any::type_name,
    cmp::Ordering,
    fmt::{Debug, Display},
};

use color_print::cformat;
use soroban_sdk::{
    testutils::Events, Address, BytesN, ConversionError, Env, Error as SorobanError, FromVal,
    InvokeError, Symbol, TryFromVal, Val,
};

use soroban_sdk::xdr::ScAddress;

pub const SYSTEM_PRECISION: u32 = 3;

pub type CallResult<T = ()> = Result<T, SorobanError>;
pub type SorobanCallResult<T, E = ConversionError> =
    Result<Result<T, E>, Result<SorobanError, InvokeError>>;

pub fn desoroban_result<T, E: Debug>(soroban_result: SorobanCallResult<T, E>) -> CallResult<T> {
    soroban_result.map(Result::unwrap).map_err(Result::unwrap)
}

pub fn float_to_int(amount: f64) -> u128 {
    (amount as f64 * 10.0f64.powi(7)) as u128
}

pub fn int_to_float(amount: u128) -> f64 {
    ((amount as f64) / 10.0f64.powi(7)) as f64
}

pub fn signed_int_to_float(amount: i128) -> f64 {
    ((amount as f64) / 10.0f64.powi(7)) as f64
}

pub fn vec_to_bytes<const N: usize>(env: &Env, bytes: Vec<u8>) -> BytesN<N> {
    let mut slice: [u8; N] = [0; N];
    slice.copy_from_slice(bytes.as_slice());

    BytesN::from_array(env, &slice)
}

pub fn format_diff<T: PartialOrd + Display>(start: T, to: T) -> String {
    match to.partial_cmp(&start).unwrap() {
        Ordering::Equal => cformat!("<dim>{start} => {to}</dim>"),
        Ordering::Greater => cformat!("<bright-green>{start} => {to}</bright-green>"),
        Ordering::Less => cformat!("<bright-red>{start} => {to}</bright-red>"),
    }
}

fn type_name_of_event<T: FromVal<Env, Val> + ?Sized>() -> String {
    static SPLITTERS: &'static [char] = &['(', ')', '[', ']', '<', '>', '{', '}', ' ', ',', '='];
    type_name::<T>()
        .split_inclusive(SPLITTERS)
        .flat_map(|component| component.rsplit("::").next())
        .collect()
}

pub fn get_latest_event<T: FromVal<Env, Val>>(env: &Env) -> Option<T> {
    env.events()
        .all()
        .iter()
        .rev()
        .find_map(|(_, topic, event_data)| {
            Symbol::try_from_val(env, &topic.last().unwrap())
                .map(|symbol| {
                    symbol
                        .to_string()
                        .eq(&type_name_of_event::<T>())
                        .then(|| T::from_val(&env, &event_data))
                })
                .ok()
                .flatten()
        })
}

pub fn assert_rel_eq(a: u128, b: u128, d: u128) {
    assert!(
        a.abs_diff(b) <= d,
        "a: {}, b: {}, d: {}, diff: {}",
        a,
        b,
        d,
        a.abs_diff(b)
    );
}

pub fn contract_id(address: &Address) -> BytesN<32> {
    let sc_address: ScAddress = address.try_into().unwrap();
    if let ScAddress::Contract(c) = sc_address {
        BytesN::from_array(address.env(), &c.0)
    } else {
        panic!("address is not a contract {:?}", address);
    }
}