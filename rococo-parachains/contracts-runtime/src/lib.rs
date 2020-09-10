// Copyright 2019 Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use cumulus_pallet_contracts_rpc_runtime_api::ContractExecResult;
use rococo_parachain_primitives::*;
use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, IdentityLookup, Saturating},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use xcm_executor::{
	XcmExecutor, Config, CurrencyAdapter,
	traits::{NativeAsset, IsConcrete},
};
use xcm::v0::{MultiLocation, MultiNetwork}; // TODO, could move this to `xcm_executor`

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::Randomness,
	weights::{constants::WEIGHT_PER_SECOND, IdentityFee, Weight},
	StorageValue,
};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

pub type SessionHandlers = ();

impl_opaque_keys! {
	pub struct SessionKeys {}
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("cumulus-contracts-parachain"),
	impl_name: create_runtime_str!("cumulus-contracts-parachain"),
	authoring_version: 1,
	spec_version: 4,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

pub const EPOCH_DURATION_IN_BLOCKS: u32 = 10 * MINUTES;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

#[derive(codec::Encode, codec::Decode)]
pub enum XCMPMessage<XAccountId, XBalance> {
	/// Transfer tokens to the given account from the Parachain account.
	TransferToken(XAccountId, XBalance),
}

/// The version infromation used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
	/// Assume 10% of weight for average on_initialize calls.
	pub MaximumExtrinsicWeight: Weight = AvailableBlockRatio::get()
		.saturating_sub(Perbill::from_percent(10)) * MaximumBlockWeight::get();
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
	pub const ExtrinsicBaseWeight: Weight = 10_000_000;
}

impl frame_system::Trait for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Maximum weight of each block. With a default weight system of 1byte == 1weight, 4mb is ok.
	type MaximumBlockWeight = MaximumBlockWeight;
	/// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
	type MaximumBlockLength = MaximumBlockLength;
	/// Portion of the block weight that is available to all normal transactions.
	type AvailableBlockRatio = AvailableBlockRatio;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type ModuleToIndex = ModuleToIndex;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
	type BlockExecutionWeight = ();
	type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const TransferFee: u128 = 0;
	pub const CreationFee: u128 = 0;
	pub const TransactionByteFee: u128 = 1;
}

impl pallet_balances::Trait for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

impl pallet_transaction_payment::Trait for Runtime {
	type Currency = Balances;
	type OnTransactionPayment = ();
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Trait for Runtime {
	type Call = Call;
	type Event = Event;
}

impl cumulus_parachain_upgrade::Trait for Runtime {
	type Event = Event;
	type OnValidationFunctionParams = ();
}

parameter_types! {
	pub const RocLocation: MultiLocation = MultiLocation::X1(Junction::Parent);
	pub const PolkadotNetwork: MultiNetwork = MultiNetwork::Polkadot;
}

use polkadot_parachain::primitives::{AccountIdConversion, Id as ParaId, Sibling};
use xcm::v0::{MultiOrigin, Junction};
use xcm_executor::traits::{PunnFromLocation, PunnIntoLocation, ConvertOrigin};
use codec::Encode;

// TODO: Maybe make something generic for this.
pub struct LocalPunner;
impl PunnFromLocation<AccountId> for LocalPunner {
	fn punn_from_location(location: &MultiLocation) -> Option<AccountId> {
		Some(match location {
			MultiLocation::X1(Junction::Parent) => AccountId::default(),
			MultiLocation::X2(Junction::Parent, Junction::Parachain { id }) => Sibling((*id).into()).into_account(),
			MultiLocation::X1(Junction::AccountId32 { id, network: MultiNetwork::Polkadot }) |
			MultiLocation::X1(Junction::AccountId32 { id, network: MultiNetwork::Any }) => (*id).into(),
			x => ("multiloc", x).using_encoded(sp_io::hashing::blake2_256).into(),
		})
	}
}
impl PunnIntoLocation<AccountId> for LocalPunner {
	fn punn_into_location(who: AccountId) -> Option<MultiLocation> {
		if who == AccountId::default() {
			return Some(Junction::Parent.into())
		}
		if let Some(id) = Sibling::try_from_account(&who) {
			return Some(MultiLocation::X2(Junction::Parent, Junction::Parachain { id: id.0.into() }))
		}
		Some(Junction::AccountId32 { id: who.into(), network: MultiNetwork::Polkadot }.into())
	}
}

pub type LocalAssetTransactor =
	CurrencyAdapter<
		// Use this currency:
		Balances,
		// Use this currency when it is a fungible asset matching the given location or name:
		IsConcrete<RocLocation>,
		// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
		LocalPunner,
		// Our chain's account ID type (we can't get away without mentioning it explicitly):
		AccountId,
	>;
pub struct LocalOriginConverter;
impl ConvertOrigin<Origin> for LocalOriginConverter {
	fn convert_origin(origin: MultiLocation, kind: MultiOrigin) -> Result<Origin, xcm::v0::Error> {
		Ok(match (kind, origin) {
			// Sovereign accounts are handled by our `LocalPunner`.
			(MultiOrigin::SovereignAccount, origin)
				=> frame_system::RawOrigin::Signed(LocalPunner::punn_from_location(&origin).ok_or(())?).into(),

			// Our Relay-chain has a native origin.
			(MultiOrigin::Native, MultiLocation::X1(Junction::Parent))
				=> cumulus_message_broker::Origin::Relay.into(),

			// Sibling Parachains have a native origin.
			(MultiOrigin::Native, MultiLocation::X2(Junction::Parent, Junction::Parachain { id }))
				=> cumulus_message_broker::Origin::SiblingParachain(id.into()).into(),

			// AccountIds for either Polkadot or "Any" network are treated literally.
			(MultiOrigin::Native, MultiLocation::X1(Junction::AccountId32 { id, network: MultiNetwork::Polkadot })) |
			(MultiOrigin::Native, MultiLocation::X1(Junction::AccountId32 { id, network: MultiNetwork::Any })) => frame_system::RawOrigin::Signed(id.into()).into(),

			// We assume that system parahains and the relay chain both run with Root privs:
			(MultiOrigin::Superuser, MultiLocation::X2(Junction::Parent, Junction::Parachain { id })) if ParaId::from(id).is_system()
				=> frame_system::RawOrigin::Root.into(),
			(MultiOrigin::Superuser, MultiLocation::X1(Junction::Parent)) => frame_system::RawOrigin::Root.into(),
			_ => Err(())?,
		})
	}
}

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = MessageBroker;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
}

impl cumulus_message_broker::Trait for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ParachainId = ParachainInfo;
}

impl cumulus_xcm_handler::Trait for Runtime {
	type Event = Event;
	type AccountIdConverter = LocalPunner;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl parachain_info::Trait for Runtime {}

// We disable the rent system for easier testing.
parameter_types! {
	pub const TombstoneDeposit: Balance = 0;
	pub const RentByteFee: Balance = 0;
	pub const RentDepositOffset: Balance = 0;
	pub const SurchargeReward: Balance = 0;
}

impl cumulus_pallet_contracts::Trait for Runtime {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = Balances;
	type Call = Call;
	type Event = Event;
	type DetermineContractAddress = cumulus_pallet_contracts::SimpleAddressDeterminer<Runtime>;
	type TrieIdGenerator = cumulus_pallet_contracts::TrieIdFromParentCounter<Runtime>;
	type RentPayment = ();
	type SignedClaimHandicap = cumulus_pallet_contracts::DefaultSignedClaimHandicap;
	type TombstoneDeposit = TombstoneDeposit;
	type StorageSizeOffset = cumulus_pallet_contracts::DefaultStorageSizeOffset;
	type RentByteFee = RentByteFee;
	type RentDepositOffset = RentDepositOffset;
	type SurchargeReward = SurchargeReward;
	type MaxDepth = cumulus_pallet_contracts::DefaultMaxDepth;
	type MaxValueSize = cumulus_pallet_contracts::DefaultMaxValueSize;
	type WeightPrice = pallet_transaction_payment::Module<Self>;
}

construct_runtime! {
	pub enum Runtime where
		Block = Block,
		NodeBlock = rococo_parachain_primitives::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Storage, Config, Event<T>},
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		Contracts: cumulus_pallet_contracts::{Module, Call, Config, Storage, Event<T>},
		Sudo: pallet_sudo::{Module, Call, Storage, Config<T>, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
		ParachainUpgrade: cumulus_parachain_upgrade::{Module, Call, Storage, Inherent, Event},
		MessageBroker: cumulus_message_broker::{Module, Call, Inherent, Event<T>, Origin},
		XcmHandler: cumulus_xcm_handler::{Module, Call, Event},
		TransactionPayment: pallet_transaction_payment::{Module, Storage},
		ParachainInfo: parachain_info::{Module, Storage, Config},
	}
}

/// The address format for describing accounts.
pub type Address = AccountId;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllModules,
>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(
			extrinsic: <Block as BlockT>::Extrinsic,
		) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}

		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}
	}

	impl cumulus_pallet_contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance, BlockNumber>
		for Runtime
	{
		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: u64,
			input_data: Vec<u8>,
		) -> ContractExecResult {
			let (exec_result, gas_consumed) =
				Contracts::bare_call(origin, dest.into(), value, gas_limit, input_data);
			match exec_result {
				Ok(v) => ContractExecResult::Success {
					flags: v.status.into(),
					data: v.data,
					gas_consumed: gas_consumed,
				},
				Err(_) => ContractExecResult::Error,
			}
		}

		fn get_storage(
			address: AccountId,
			key: [u8; 32],
		) -> cumulus_pallet_contracts_primitives::GetStorageResult {
			Contracts::get_storage(address, key)
		}

		fn rent_projection(
			address: AccountId,
		) -> cumulus_pallet_contracts_primitives::RentProjectionResult<BlockNumber> {
			Contracts::rent_projection(address)
		}
	}
}

cumulus_runtime::register_validate_block!(Block, Executive);
