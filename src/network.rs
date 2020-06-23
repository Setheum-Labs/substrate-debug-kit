use crate::primitives::{runtime, AccountId, Balance, Hash, Nonce};
use crate::{storage, Client};
use atomic_refcell::AtomicRefCell as RefCell;
use codec::Decode;
use frame_support::{Blake2_128Concat, Twox64Concat};
use frame_system::AccountInfo;
use jsonrpsee::common::{to_value as to_json_value, Params};
use pallet_balances::AccountData;
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use sp_core::storage::{StorageData, StorageKey};
use sp_runtime::traits::Convert;

static ISSUANCE: RefCell<Balance> = RefCell::new(0);

/// Deals with total issuance
pub mod issuance {
	use super::{get_total_issuance, ISSUANCE};
	use crate::{Balance, Client, Hash};

	/// Get the previously set total issuance.
	pub fn get() -> Balance {
		ISSUANCE.borrow().clone()
	}

	/// Set the total issuance. Any code wanting to use `CurrencyToVoteHandler` must call this first
	/// to set correct value in the global pointer.
	pub async fn set(client: &Client, at: Hash) {
		let total_issuance = get_total_issuance(client, at).await;
		*ISSUANCE.borrow_mut() = total_issuance;
	}
}

pub struct CurrencyToVoteHandler;
impl CurrencyToVoteHandler {
	fn factor() -> u128 {
		(issuance::get() / u64::max_value() as u128).max(1)
	}
}

impl Convert<u128, u64> for CurrencyToVoteHandler {
	fn convert(x: Balance) -> u64 {
		(x / Self::factor()) as u64
	}
}

impl Convert<u128, u128> for CurrencyToVoteHandler {
	fn convert(x: u128) -> Balance {
		x * Self::factor()
	}
}

/// Get the nick of a given account id.
///
/// seemingly DEPRECATED.
#[allow(dead_code)]
pub async fn get_nick(who: &AccountId, client: &Client, at: Hash) -> String {
	let nick = storage::read::<(Vec<u8>, Balance)>(
		storage::map_key::<Twox64Concat>(b"Nicks", b"NameOf", who.as_ref()),
		client,
		at,
	)
	.await;

	if nick.is_some() {
		String::from_utf8(nick.unwrap().0).unwrap()
	} else {
		String::from("[NO_NICK]")
	}
}

pub async fn get_identity(who: &AccountId, client: &Client, at: Hash) -> String {
	use pallet_identity::{Data, Registration};
	let maybe_identity = storage::read::<Registration<Balance>>(
		storage::map_key::<Twox64Concat>(b"Identity", b"IdentityOf", who.as_ref()),
		client,
		at,
	)
	.await;

	if let Some(identity) = maybe_identity {
		let info = identity.info;
		let display = info.display;

		match display {
			Data::Raw(bytes) => String::from_utf8(bytes).expect("Identity not utf-8"),
			_ => "OPAQUE_IDENTITY".to_string(),
		}
	} else {
		"NO_IDENT".to_string()
	}
}

/// Get the latest finalized head of the chain.
pub async fn get_head(client: &Client) -> Hash {
	let data: Option<StorageData> = client
		.request("chain_getFinalizedHead", Params::None)
		.await
		.expect("get chain finalized head request failed");
	let now_raw = data.expect("Should always get the head hash").0;
	<Hash as Decode>::decode(&mut &*now_raw).expect("Block hash should decode")
}

/// Get the block at a particular hash
pub async fn get_block(client: &Client, at: Hash) -> runtime::SignedBlock {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let data: Option<runtime::SignedBlock> = client
		.request("chain_getBlock", Params::Array(vec![at]))
		.await
		.expect("Failed to decode block");

	data.unwrap()
}

/// Get the runtime version at the given block.
pub async fn get_runtime_version(client: &Client, at: Hash) -> sp_version::RuntimeVersion {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let data: Option<sp_version::RuntimeVersion> = client
		.request("state_getRuntimeVersion", Params::Array(vec![at]))
		.await
		.expect("Failed to decode block");

	data.unwrap()
}

pub async fn get_metadata(client: &Client, at: Hash) -> sp_core::Bytes {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let data: Option<sp_core::Bytes> = client
		.request("state_getMetadata", Params::Array(vec![at]))
		.await
		.expect("Failed to decode block");

	data.unwrap()
}

/// Get the extrinsic info
pub async fn query_info(
	extrinsic: sp_core::Bytes,
	client: &Client,
	at: Hash,
) -> RuntimeDispatchInfo<Balance> {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let extrinsic = to_json_value(extrinsic).expect("extrinsic serialization infallible");
	let data: Option<RuntimeDispatchInfo<Balance>> = client
		.request("payment_queryInfo", Params::Array(vec![extrinsic, at]))
		.await
		.unwrap();

	data.unwrap()
}

pub async fn got_storage_size(key: StorageKey, client: &Client, at: Hash) -> Option<u64> {
	let at = to_json_value(at).expect("Block hash serialization infallible");
	let key = to_json_value(key).expect("extrinsic serialization infallible");
	client
		.request("state_getStorageSize", Params::Array(vec![key, at]))
		.await
		.unwrap()
}

pub async fn get_events_at(
	client: &Client,
	at: Hash,
) -> Option<Vec<frame_system::EventRecord<runtime::Event, Hash>>> {
	let key = storage::value_key(b"System", b"Events");
	storage::read(key, client, at).await
}

pub async fn get_account_data_at(
	account: &[u8],
	client: &Client,
	at: Hash,
) -> AccountInfo<Nonce, AccountData<Balance>> {
	storage::read::<AccountInfo<Nonce, AccountData<Balance>>>(
		storage::map_key::<Blake2_128Concat>(b"System", b"Account", account),
		client,
		at,
	)
	.await
	.unwrap()
}

/// Get total issuance of the chain.
async fn get_total_issuance(client: &Client, at: Hash) -> Balance {
	let maybe_total_issuance = storage::read::<Balance>(
		storage::value_key(b"Balances", b"TotalIssuance"),
		&client,
		at,
	)
	.await;

	maybe_total_issuance.unwrap_or(0)
}
