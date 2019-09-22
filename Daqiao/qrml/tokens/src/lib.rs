#![cfg_attr(not(feature = "std"), no_std)]

extern crate srml_support as support;
extern crate sr_primitives as runtime_primitives;
extern crate parity_codec;
extern crate srml_system as system;
extern crate sr_std;


use support::{decl_module, decl_storage, decl_event, StorageMap, StorageValue, dispatch::Result, Parameter, ensure};
use runtime_primitives::traits::{SimpleArithmetic, Bounded, One, CheckedAdd, CheckedSub, As, Member};
use parity_codec::{Encode, Decode};
use system::ensure_signed;
use sr_std::prelude::*;

/// The module's configuration trait.
pub trait Trait: system::Trait {
  type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
  type TokenId: Parameter + Default + Bounded + SimpleArithmetic;
  type TokenBalance: Parameter + Member + SimpleArithmetic + Default + Copy + As<usize> + As<u64> + As<u128>;
}

type Symbol = Vec<u8>;
type TokenDesc = Vec<u8>;

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Erc20MintableBurnable<T> {
  name: Vec<u8>,
  desc: Vec<u8>,
  total_supply: T,
}

decl_storage! {
  trait Store for Module<T: Trait> as Tokens {
      TokenID get(token_id): T::TokenId;
      
      TokenTabel get(token_details): map T::TokenId => Erc20MintableBurnable<T::TokenBalance>;
      
      BalanceOf get(balance_of): map (T::TokenId, T::AccountId) => T::TokenBalance;
  }
}

decl_module! {
  /// The module declaration.
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    fn deposit_event<T>() = default;

    fn init(origin, name: Symbol, desc: TokenDesc, total_supply: T::TokenBalance) -> Result {
          let sender = ensure_signed(origin)?;

          ensure!(name.len() <= 64, "token name cannot exceed 64 bytes");
          ensure!(desc.len() <= 32, "token desc cannot exceed 32 bytes");

          let token_id = Self::token_id();
          let next_token_id = token_id.checked_add(&One::one()).ok_or("overflow in calculating next token id")?;
          <TokenID<T>>::put(next_token_id);
          
          let token = Erc20MintableBurnable {
              name,
              desc,
              total_supply,
          };

          <TokenTabel<T>>::insert(token_id.clone(), token);
          <BalanceOf<T>>::insert((token_id, sender), total_supply);

          Ok(())
      }
      
      fn transfer(_origin, token_id: T::TokenId, to: T::AccountId, value: T::TokenBalance) -> Result {
          let sender = ensure_signed(_origin)?;
          Self::_transfer(token_id, sender, to, value)
      }
      
      pub fn mint(token_id: T::TokenId, owner: T::AccountId, value: T::TokenBalance) -> Result {
        Self::_mint(token_id, owner, value)
      }
      
      pub fn burn(token_id: T::TokenId, owner: T::AccountId, value: T::TokenBalance) -> Result {
        Self::_burn(token_id, owner, value)
      }
  }
}

decl_event!(
  pub enum Event<T> where TokenId = <T as self::Trait>::TokenId, AccountId = <T as system::Trait>::AccountId, Balance = <T as self::Trait>::TokenBalance {
    Transfer(TokenId, AccountId, AccountId, Balance),
  }
);

//impl<T: Trait> Module<T> {
//  /// Deposit one of this module's events.
////  fn deposit_event(event: Event<T>) {
////    <system::Module<T>>::deposit_event(<T as Trait>::Event::from(event).into());
////  }
//}

impl<T: Trait> Module<T> {
  // internal
  fn _transfer(
    token_id: T::TokenId,
    from: T::AccountId,
    to: T::AccountId,
    value: T::TokenBalance,
  ) -> Result {
    ensure!(<BalanceOf<T>>::exists((token_id.clone(), from.clone())), "Account does not own this token");
    let sender_balance = Self::balance_of((token_id.clone(), from.clone()));
    ensure!(sender_balance >= value, "Not enough balance.");
    
    let updated_from_balance = sender_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
    let receiver_balance = Self::balance_of((token_id.clone(), to.clone()));
    let updated_to_balance = receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;
    
    // reduce sender's balance
    <BalanceOf<T>>::insert((token_id.clone(), from.clone()), updated_from_balance);
    
    // increase receiver's balance
    <BalanceOf<T>>::insert((token_id.clone(), to.clone()), updated_to_balance);
    
    Self::deposit_event(RawEvent::Transfer(token_id, from, to, value));
    Ok(())
  }
  
  fn _add_token_to_owner(token_id: T::TokenId, owner: T::AccountId, value: T::TokenBalance) -> Result {
    let owner_balance = Self::balance_of((token_id.clone(), owner.clone()));
    let updated_balance = owner_balance.checked_add(&value).ok_or("overflow in calculating balance")?;
    <BalanceOf<T>>::insert((token_id.clone(), owner.clone()), updated_balance);
    Ok(())
  }
  
  fn _sub_token_from_owner(token_id: T::TokenId, owner: T::AccountId, value: T::TokenBalance) -> Result {
    let owner_balance = Self::balance_of((token_id.clone(), owner.clone()));
    let updated_balance = owner_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
    <BalanceOf<T>>::insert((token_id.clone(), owner.clone()), updated_balance);
    Ok(())
  }
  
  fn _add_token_to_total_supply(token_id: T::TokenId, value: T::TokenBalance) -> Result {
    let token = Self::token_details(token_id.clone());
    let updated_total_supply = token.total_supply.checked_add(&value).ok_or("overflow in calculating total_supply")?;
    let updated_token = Erc20MintableBurnable {
      name: token.name,
      desc: token.desc,
      total_supply: updated_total_supply
    };
    <TokenTabel<T>>::insert(token_id.clone(), updated_token);
    Ok(())
  }
  
  fn _sub_token_from_total_supply(token_id: T::TokenId, value: T::TokenBalance) -> Result {
    let token = Self::token_details(token_id.clone());
    let updated_total_supply = token.total_supply.checked_sub(&value).ok_or("overflow in calculating total_supply")?;
    let updated_token = Erc20MintableBurnable {
      name: token.name,
      desc: token.desc,
      total_supply: updated_total_supply
    };
    <TokenTabel<T>>::insert(token_id.clone(), updated_token);
    Ok(())
  }
  
  fn _mint(token_id: T::TokenId, owner: T::AccountId, value: T::TokenBalance) -> Result {
    Self::_add_token_to_owner(token_id.clone(), owner, value.clone())?;
    Self::_add_token_to_total_supply(token_id, value)?;
    Ok(())
  }
  
  fn _burn(token_id: T::TokenId, owner: T::AccountId, value: T::TokenBalance) -> Result {
    Self::_sub_token_from_owner(token_id.clone(), owner, value.clone())?;
    Self::_sub_token_from_total_supply(token_id, value)?;
    Ok(())
  }
}

/// tests for this module
#[cfg(test)]
mod tests {
  use super::*;

  use runtime_io::with_externalities;
  use primitives::{H256, Blake2Hasher};
  use support::{impl_outer_origin, assert_ok};
  use runtime_primitives::{
    BuildStorage,
    traits::{BlakeTwo256, IdentityLookup},
    testing::{Digest, DigestItem, Header}
  };

  impl_outer_origin! {
    pub enum Origin for Test {}
  }

  // For testing the module, we construct most of a mock runtime. This means
  // first constructing a configuration type (`Test`) which `impl`s each of the
  // configuration traits of modules we want to use.
  #[derive(Clone, Eq, PartialEq)]
  pub struct Test;
  impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
  }
  impl Trait for Test {
    type Event = ();
  }
  type TemplateModule = Module<Test>;

  // This function basically just builds a genesis storage key/value store according to
  // our desired mockup.
  fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
  }

  #[test]
  fn it_works_for_default_value() {
    with_externalities(&mut new_test_ext(), || {
      // Just a dummy test for the dummy funtion `do_something`
      // calling the `do_something` function with a value 42
      assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
      // asserting that the stored value is equal to what we stored
      assert_eq!(TemplateModule::something(), Some(42));
    });
  }
}
