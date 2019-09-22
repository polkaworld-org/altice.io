// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity Ethereum.

// Parity Ethereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Ethereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Ethereum.  If not, see <http://www.gnu.org/licenses/>.

//! Authority Round parameter deserialization.
//!
//! Here is an example of input parameters where the step duration is constant at 5 seconds, the set
//! of validators is decided by the contract at address `0x10..01` starting from block 0, and where
//! the address of the contract that computes block rewards is set to `0x20..02` for blocks 0
//! through 41 and to `0x30.03` for all blocks starting from block 42.
//!
//! ```ignore
//! "params": {
//!     "stepDuration": "5",
//!     "validators": {
//!         "multi": {
//!             "0": {
//!                 "contract": "0x1000000000000000000000000000000000000001"
//!             }
//!         }
//!     },
//!     "blockRewardContractTransitions": {
//!         "0": "0x2000000000000000000000000000000000000002",
//!         "42": "0x3000000000000000000000000000000000000003"
//!     }
//! }
//! ```

use std::collections::BTreeMap;
use hash::Address;
use uint::Uint;
use bytes::Bytes;
use super::ValidatorSet;

/// Authority params deserialization.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AuthorityRoundParams {
	/// Block duration, in seconds.
	pub step_duration: Uint,
	/// Valid authorities
	pub validators: ValidatorSet,
	/// Starting step. Determined automatically if not specified.
	/// To be used for testing only.
	pub start_step: Option<Uint>,
	/// Block at which score validation should start.
	pub validate_score_transition: Option<Uint>,
	/// Block from which monotonic steps start.
	pub validate_step_transition: Option<Uint>,
	/// Whether transitions should be immediate.
	pub immediate_transitions: Option<bool>,
	/// Reward per block in wei.
	pub block_reward: Option<Uint>,
	/// Block at which the block reward contract should start being used. This option allows one to
	/// add a single block reward contract transition and is compatible with the multiple address
	/// option `block_reward_contract_transitions` below.
	pub block_reward_contract_transition: Option<Uint>,
	/// Block reward contract address which overrides the `block_reward` setting. This option allows
	/// one to add a single block reward contract address and is compatible with the multiple
	/// address option `block_reward_contract_transitions` below.
	pub block_reward_contract_address: Option<Address>,
	/// Block reward contract addresses with their associated starting block numbers.
	///
	/// Setting the block reward contract overrides `block_reward`. If the single block reward
	/// contract address is also present then it is added into the map at the block number stored in
	/// `block_reward_contract_transition` or 0 if that block number is not provided. Therefore both
	/// a single block reward contract transition and a map of reward contract transitions can be
	/// used simulataneously in the same configuration. In such a case the code requires that the
	/// block number of the single transition is strictly less than any of the block numbers in the
	/// map.
	pub block_reward_contract_transitions: Option<BTreeMap<Uint, Address>>,
	/// Block reward code. This overrides the block reward contract address.
	pub block_reward_contract_code: Option<Bytes>,
	/// Block at which maximum uncle count should be considered.
	pub maximum_uncle_count_transition: Option<Uint>,
	/// Maximum number of accepted uncles.
	pub maximum_uncle_count: Option<Uint>,
	/// Block at which empty step messages should start.
	pub empty_steps_transition: Option<Uint>,
	/// Maximum number of accepted empty steps.
	pub maximum_empty_steps: Option<Uint>,
	/// Strict validation of empty steps transition block.
	pub strict_empty_steps_transition: Option<Uint>,
	/// First block for which a 2/3 quorum (instead of 1/2) is required.
	pub two_thirds_majority_transition: Option<Uint>,
}

/// Authority engine deserialization.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityRound {
	/// Ethash params.
	pub params: AuthorityRoundParams,
}

#[cfg(test)]
mod tests {
	use ethereum_types::{U256, H160};
	use uint::Uint;
	use serde_json;
	use hash::Address;
	use spec::validator_set::ValidatorSet;
	use spec::authority_round::AuthorityRound;
	use std::str::FromStr;

	#[test]
	fn authority_round_deserialization() {
		let s = r#"{
			"params": {
				"stepDuration": "0x02",
				"validators": {
					"list" : ["0xc6d9d2cd449a754c494264e1809c50e34d64562b"]
				},
				"startStep" : 24,
				"validateStepTransition": 150,
				"blockReward": 5000000,
				"maximumUncleCountTransition": 10000000,
				"maximumUncleCount": 5
			}
		}"#;

		let deserialized: AuthorityRound = serde_json::from_str(s).unwrap();
		assert_eq!(deserialized.params.step_duration, Uint(U256::from(0x02)));
		assert_eq!(
			deserialized.params.validators,
			ValidatorSet::List(vec![Address(H160::from_str("c6d9d2cd449a754c494264e1809c50e34d64562b").unwrap())]),
		);
		assert_eq!(deserialized.params.start_step, Some(Uint(U256::from(24))));
		assert_eq!(deserialized.params.immediate_transitions, None);
		assert_eq!(deserialized.params.maximum_uncle_count_transition, Some(Uint(10_000_000.into())));
		assert_eq!(deserialized.params.maximum_uncle_count, Some(Uint(5.into())));

	}
}
