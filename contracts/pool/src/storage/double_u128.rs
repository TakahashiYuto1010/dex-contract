use soroban_sdk::contracttype;

use core::{
    fmt::Debug,
    ops::{Index, IndexMut},
};

use super::pool::Token;

#[contracttype]
#[derive(Debug, Clone, Default)]
pub struct DoubleU128 {
    pub data: (u128, u128),
}

impl DoubleU128 {
    pub fn to_array(&self) -> [u128; 2] {
        [self.data.0, self.data.1]
    }
}

impl Index<usize> for DoubleU128 {
    type Output = u128;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.data.0,
            1 => &self.data.1,
            _ => panic!("Unexpected index"),
        }
    }
}

impl IndexMut<usize> for DoubleU128 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.data.0,
            1 => &mut self.data.1,
            _ => panic!("Unexpected index"),
        }
    }
}

impl Index<Token> for DoubleU128 {
    type Output = u128;

    fn index(&self, index: Token) -> &Self::Output {
        &self[index as usize]
    }
}

impl IndexMut<Token> for DoubleU128 {
    fn index_mut(&mut self, index: Token) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

impl From<[u128; 2]> for DoubleU128 {
    #[inline]
    fn from(value: [u128; 2]) -> Self {
        Self {
            data: (value[0], value[1]),
        }
    }
}

impl From<(u128, u128)> for DoubleU128 {
    #[inline]
    fn from(data: (u128, u128)) -> Self {
        Self { data }
    }
}