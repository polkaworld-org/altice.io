
extern crate qrml_tokens as tokens;

use support::{decl_module, decl_storage, decl_event, StorageMap, StorageValue, dispatch::Result, ensure};
use runtime_primitives::traits::{CheckedSub, Zero};
use parity_codec::{Encode, Decode};
use system::ensure_signed;
use rstd::prelude::*;

pub type TokenId<T> = <T as tokens::Trait>::TokenId;
// pub type ChainId = u32;
// pub type ExtTxID = Vec<u8>;
//pub type Hash = primitives::H256;

pub trait Trait: system::Trait + tokens::Trait {
  type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct PledgeInfo <U, V>{
  chain_id: u32,
  ext_txid: Vec<u8>,
  account_id: U,
  pledge_amount: V,
  can_withdraw: bool,
  withdraw_history: Vec<Vec<u8>>,
  withdraw_address: Vec<u8>
}

decl_storage! {
  trait Store for Module<T: Trait> as Daqiao {
    DummyValue get(dummy_value): u32;
    // 外链id => token id
    ChainToken get(chain_token): map u32 => Option<T::TokenId>;

    // 外链质押txid => PledgeInfo
    PledgeRecords get(pledge_records): map Vec<u8> => PledgeInfo<T::AccountId, T::TokenBalance>;
  }
}

decl_module! {
  /// The module declaration.
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    fn deposit_event<T>() = default;

    // 关联ChainId和TokenId
    pub fn register(origin, chain_id: u32, token_id: T::TokenId) -> Result {
      Self::_register(origin, chain_id, token_id)
    }

    // 质押
    pub fn pledge(origin, chain_id: u32, ext_txid: Vec<u8>, amount: T::TokenBalance, reciever: T::AccountId) -> Result {
      Self::_pledge(origin, chain_id, ext_txid, amount, reciever)
    }

    // 提现
    pub fn withdraw(origin, chain_id: u32, ext_txid: Vec<u8>, ext_address: Vec<u8>) -> Result {
      Self::_withdraw(origin, chain_id, ext_txid, ext_address)
    }

    pub fn dummy(origin, x: u32, chain_id: u32) -> Result {
      <DummyValue<T>>::put(x + chain_id as u32);
      Self::deposit_event(RawEvent::DummyCalled(x));
      Ok(())
    }
  }
}

decl_event!(
  pub enum Event<T> where AccountId = <T as system::Trait>::AccountId, Balance = <T as tokens::Trait>::TokenBalance {
    Pledged(u32, Vec<u8>, AccountId, Balance),
    Withdrawn(u32, Vec<u8>, AccountId, Balance, Vec<u8>),
    DummyCalled(u32),
  }
);

impl<T: Trait> Module<T> {

  fn _register(origin: T::Origin, chain_id: u32, token_id: T::TokenId) -> Result {
    let _ = ensure_signed(origin)?;
    ensure!(!<ChainToken<T>>::exists(chain_id.clone()), "ChainId already exists.");
    <ChainToken<T>>::insert(chain_id, token_id);
    Ok(())
  }


  fn _pledge(origin: T::Origin, chain_id: u32, ext_txid: Vec<u8>, amount: T::TokenBalance, reciever: T::AccountId) -> Result {
    let sender = ensure_signed(origin)?;
    // TODO 验证 sender在admin list中

    let token_id = match Self::chain_token(chain_id.clone()) {
      Some(t) => t,
      None => return Err("Chain id not exists.")
    };
    // 验证交易id是否被pledge过了
    ensure!(!<PledgeRecords<T>>::exists(ext_txid.clone()), "ext_txid already pledged");

    let pi = PledgeInfo {
      chain_id: chain_id.clone(),
      ext_txid: ext_txid.clone(),
      account_id: reciever.clone(),
      pledge_amount: amount,
      can_withdraw: true,
      withdraw_history: Vec::new(),
      withdraw_address: Vec::new(),
    };

    // mint
     <tokens::Module<T>>::mint(token_id.clone(), reciever.clone(), amount.clone())?;

    // store pledge history
     <PledgeRecords<T>>::insert(ext_txid.clone(), pi);

     Self::deposit_event(RawEvent::Pledged(chain_id, ext_txid, reciever, amount));
    Ok(())
  }
  
  fn _withdraw(origin: T::Origin, chain_id: u32, ext_txid: Vec<u8>, ext_address: Vec<u8>) -> Result {
    let sender = ensure_signed(origin) ?;
    ensure!(<PledgeRecords<T>>::exists(ext_txid.clone()), "This ext_txid PledgeRecords does not exist");

    let token_id = match Self::chain_token(chain_id.clone()) {
      Some(t) => t,
      None => return Err("Chain id not exists.")
    };
    let mut pi = <PledgeRecords<T>>::get(ext_txid.clone());
    let amount = pi.pledge_amount;
    pi.withdraw_address = ext_address.clone();

    // 检查余额是否足够
    let balance = <tokens::Module<T>>::balance_of(&(token_id.clone(), sender.clone()));
    let _ = balance.checked_sub(&amount).ok_or("Not sufficient balance for withdraw")?;
    
    ensure!(pi.can_withdraw && pi.account_id == sender.clone(), "Withdraw failed.");

    pi.can_withdraw = false;
    pi.pledge_amount = Zero::zero();

    <PledgeRecords<T>>::insert(ext_txid.clone(), pi);
    <tokens::Module<T>>::burn(token_id, sender.clone(), amount)?;
    Self::deposit_event(RawEvent::Withdrawn(chain_id, ext_txid, sender, amount, ext_address));
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
  type Daqiao = Module<Test>;

  // This function basically just builds a genesis storage key/value store according to
  // our desired mockup.
  fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
  }

}
