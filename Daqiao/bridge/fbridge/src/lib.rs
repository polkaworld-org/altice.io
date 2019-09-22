#![cfg_attr(not(feature = "std"), no_std)]
pub mod fbridge;
use rstd::prelude::*;
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
