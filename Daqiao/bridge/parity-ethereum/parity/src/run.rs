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

use std::any::Any;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use std::thread;

use ansi_term::Colour;
//use bytes::Bytes;
//use call_contract::CallContract;
//use client_traits::{BlockInfo, BlockChainClient};
use ethcore::client::{/*Client, */DatabaseCompactionProfile, VMType};
//use ethcore::miner::{/*self, */stratum, /*Miner, MinerService, */MinerOptions};
use snapshot::{/*self, */SnapshotConfiguration};
use spec::SpecParams;
use verification::queue::VerifierSettings;
use ethcore_logger::{Config as LogConfig, RotatingLogger};
//use ethcore_service::ClientService;
//use ethereum_types::Address;
//use futures::{IntoFuture, Stream};
use hash_fetch::{/*self, */fetch};

use crate::informant::{Informant, LightNodeInformantData/*, FullNodeInformantData*/};
//use journaldb::Algorithm;
use light::Cache as LightDataCache;
//use miner::external::ExternalMiner;
//use miner::work_notify::WorkPoster;
//use node_filter::NodeFilter;
use parity_runtime::Runtime;
use sync::{self/*, SyncConfig, PrivateTxHandler*/};
use types::{
	client_types::Mode,
	engines::OptimizeFor,
//	ids::BlockId,
//	snapshot::Snapshotting,
};
use parity_rpc::{
	Origin, Metadata, NetworkSettings, informant, PubSubSession, FutureResult, FutureResponse, FutureOutput
};
use updater::{UpdatePolicy/*, Updater*/};
use parity_version::version;
use ethcore_private_tx::{ProviderConfig, EncryptorConfig/*, SecretStoreEncryptor*/};
use crate::params::{
	SpecType, Pruning, AccountsConfig, GasPricerConfig, /*MinerExtras, */Switch//,
//	tracing_switch_to_bool, fatdb_switch_to_bool, mode_switch_to_bool
};
use crate::account_utils;
use crate::helpers::{/*to_client_config, */execute_upgrades, passwords_from_files};
use dir::{Directories, DatabaseDirectories};
use crate::cache::CacheConfig;
use crate::user_defaults::UserDefaults;
use crate::ipfs;
use jsonrpc_core;
//use modules;
//use registrar::{RegistrarClient, Asynchronous};
use crate::rpc;
use crate::rpc_apis;
use crate::secretstore;
use crate::signer;
use crate::db;

// how often to take periodic snapshots.
//const SNAPSHOT_PERIOD: u64 = 5000;

// how many blocks to wait before starting a periodic snapshot.
//const SNAPSHOT_HISTORY: u64 = 100;

// Number of minutes before a given gas price corpus should expire.
// Light client only.
const GAS_CORPUS_EXPIRATION_MINUTES: u64 = 60 * 6;

// Full client number of DNS threads
//const FETCH_FULL_NUM_DNS_THREADS: usize = 4;

// Light client number of DNS threads
const FETCH_LIGHT_NUM_DNS_THREADS: usize = 1;

#[derive(Debug, PartialEq)]
pub struct RunCmd {
	pub cache_config: CacheConfig,
	pub dirs: Directories,
	pub spec: SpecType,
	pub pruning: Pruning,
	pub pruning_history: u64,
	pub pruning_memory: usize,
	/// Some if execution should be daemonized. Contains pid_file path.
	pub daemon: Option<String>,
	pub logger_config: LogConfig,
//	pub miner_options: MinerOptions,
	pub gas_price_percentile: usize,
	pub poll_lifetime: u32,
	pub ws_conf: rpc::WsConfiguration,
	pub http_conf: rpc::HttpConfiguration,
	pub ipc_conf: rpc::IpcConfiguration,
	pub net_conf: sync::NetworkConfiguration,
	pub network_id: Option<u64>,
	pub warp_sync: bool,
	pub warp_barrier: Option<u64>,
	pub acc_conf: AccountsConfig,
	pub gas_pricer_conf: GasPricerConfig,
//	pub miner_extras: MinerExtras,
	pub update_policy: UpdatePolicy,
	pub mode: Option<Mode>,
	pub tracing: Switch,
	pub fat_db: Switch,
	pub compaction: DatabaseCompactionProfile,
	pub vm_type: VMType,
	pub geth_compatibility: bool,
	pub experimental_rpcs: bool,
	pub net_settings: NetworkSettings,
	pub ipfs_conf: ipfs::Configuration,
	pub secretstore_conf: secretstore::Configuration,
	pub private_provider_conf: ProviderConfig,
	pub private_encryptor_conf: EncryptorConfig,
	pub private_tx_enabled: bool,
	pub name: String,
	pub custom_bootnodes: bool,
//	pub stratum: Option<stratum::Options>,
	pub snapshot_conf: SnapshotConfiguration,
	pub check_seal: bool,
	pub allow_missing_blocks: bool,
	pub download_old_blocks: bool,
	pub verifier_settings: VerifierSettings,
	pub serve_light: bool,
	pub light: bool,
	pub no_persistent_txqueue: bool,
	pub no_hardcoded_sync: bool,
	pub max_round_blocks_to_import: usize,
	pub on_demand_response_time_window: Option<u64>,
	pub on_demand_request_backoff_start: Option<u64>,
	pub on_demand_request_backoff_max: Option<u64>,
	pub on_demand_request_backoff_rounds_max: Option<usize>,
	pub on_demand_request_consecutive_failures: Option<usize>,
}

// node info fetcher for the local store.
//struct FullNodeInfo {
//	miner: Option<Arc<Miner>>, // TODO: only TXQ needed, just use that after decoupling.
//}

//impl ::local_store::NodeInfo for FullNodeInfo {
//	fn pending_transactions(&self) -> Vec<::types::transaction::PendingTransaction> {
//		let miner = match self.miner.as_ref() {
//			Some(m) => m,
//			None => return Vec::new(),
//		};
//
//		miner.local_transactions()
//			.values()
//			.filter_map(|status| match *status {
//				::miner::pool::local_transactions::Status::Pending(ref tx) => Some(tx.pending().clone()),
//				_ => None,
//			})
//			.collect()
//	}
//}

type LightClient = ::light::client::Client<crate::light_helpers::EpochFetch>;

// helper for light execution.
fn execute_light_impl<Cr>(cmd: RunCmd, logger: Arc<RotatingLogger>, on_client_rq: Cr) -> Result<RunningClient, String>
	where Cr: Fn(String) + 'static + Send
{
	use light::client as light_client;
	use sync::{LightSyncParams, LightSync, ManageNetwork};
	use parking_lot::{Mutex, RwLock};

	println!("IN execute_light_impl");
	// load spec
	let spec = cmd.spec.spec(SpecParams::new(cmd.dirs.cache.as_ref(), OptimizeFor::Memory))?;

	// load genesis hash
	let genesis_hash = spec.genesis_header().hash();

	// database paths
	let db_dirs = cmd.dirs.database(genesis_hash, cmd.spec.legacy_fork_name(), spec.data_dir.clone());

	// user defaults path
	let user_defaults_path = db_dirs.user_defaults_path();

	// load user defaults
	let user_defaults = UserDefaults::load(&user_defaults_path)?;

	// select pruning algorithm
	let algorithm = cmd.pruning.to_algorithm(&user_defaults);

	// execute upgrades
	execute_upgrades(&cmd.dirs.base, &db_dirs, algorithm, &cmd.compaction)?;

	// create dirs used by parity
	cmd.dirs.create_dirs(cmd.acc_conf.unlocked_accounts.len() == 0, cmd.secretstore_conf.enabled)?;

	//print out running parity environment
	print_running_environment(&spec.data_dir, &cmd.dirs, &db_dirs);

	info!("Running in experimental {} mode.", Colour::Blue.bold().paint("Light Client"));

	// TODO: configurable cache size.
	let cache = LightDataCache::new(Default::default(), Duration::from_secs(60 * GAS_CORPUS_EXPIRATION_MINUTES));
	let cache = Arc::new(Mutex::new(cache));

	// start client and create transaction queue.
	let mut config = light_client::Config {
		queue: Default::default(),
		chain_column: ::ethcore_db::COL_LIGHT_CHAIN,
		verify_full: true,
		check_seal: cmd.check_seal,
		no_hardcoded_sync: cmd.no_hardcoded_sync,
	};

	config.queue.max_mem_use = cmd.cache_config.queue() as usize * 1024 * 1024;
	config.queue.verifier_settings = cmd.verifier_settings;

	// start on_demand service.

	let response_time_window = cmd.on_demand_response_time_window.map_or(
		::light::on_demand::DEFAULT_RESPONSE_TIME_TO_LIVE,
		|s| Duration::from_secs(s)
	);

	let request_backoff_start = cmd.on_demand_request_backoff_start.map_or(
		::light::on_demand::DEFAULT_REQUEST_MIN_BACKOFF_DURATION,
		|s| Duration::from_secs(s)
	);

	let request_backoff_max = cmd.on_demand_request_backoff_max.map_or(
		::light::on_demand::DEFAULT_REQUEST_MAX_BACKOFF_DURATION,
		|s| Duration::from_secs(s)
	);

	let on_demand = Arc::new({
		::light::on_demand::OnDemand::new(
			cache.clone(),
			response_time_window,
			request_backoff_start,
			request_backoff_max,
			cmd.on_demand_request_backoff_rounds_max.unwrap_or(::light::on_demand::DEFAULT_MAX_REQUEST_BACKOFF_ROUNDS),
			cmd.on_demand_request_consecutive_failures.unwrap_or(::light::on_demand::DEFAULT_NUM_CONSECUTIVE_FAILED_REQUESTS)
		)
	});

	let sync_handle = Arc::new(RwLock::new(Weak::new()));
	let fetch = crate::light_helpers::EpochFetch {
		on_demand: on_demand.clone(),
		sync: sync_handle.clone(),
	};

	// initialize database.
	let db = db::open_db(&db_dirs.client_path(algorithm).to_str().expect("DB path could not be converted to string."),
						 &cmd.cache_config,
						 &cmd.compaction).map_err(|e| format!("Failed to open database {:?}", e))?;

	let service = light_client::Service::start(config, &spec, fetch, db, cache.clone())
		.map_err(|e| format!("Error starting light client: {}", e))?;
	let client = service.client().clone();
	let txq = Arc::new(RwLock::new(::light::transaction_queue::TransactionQueue::default()));
	let provider = ::light::provider::LightProvider::new(client.clone(), txq.clone());

	// start network.
	// set up bootnodes
	let mut net_conf = cmd.net_conf;
	if !cmd.custom_bootnodes {
		net_conf.boot_nodes = spec.nodes.clone();
	}

	// set network path.
	net_conf.net_config_path = Some(db_dirs.network_path().to_string_lossy().into_owned());
	let sync_params = LightSyncParams {
		network_config: net_conf.into_basic().map_err(|e| format!("Failed to produce network config: {}", e))?,
		client: Arc::new(provider),
		network_id: cmd.network_id.unwrap_or(spec.network_id()),
		subprotocol_name: sync::LIGHT_PROTOCOL,
		handlers: vec![on_demand.clone()],
	};
	let light_sync = LightSync::new(sync_params).map_err(|e| format!("Error starting network: {}", e))?;
	let light_sync = Arc::new(light_sync);
	*sync_handle.write() = Arc::downgrade(&light_sync);

	// spin up event loop
	let runtime = Runtime::with_default_thread_count();

	// start the network.
	light_sync.start_network();

	// fetch service
	let fetch = fetch::Client::new(FETCH_LIGHT_NUM_DNS_THREADS).map_err(|e| format!("Error starting fetch client: {:?}", e))?;
	let passwords = passwords_from_files(&cmd.acc_conf.password_files)?;

	// prepare account provider
	let account_provider = Arc::new(account_utils::prepare_account_provider(&cmd.spec, &cmd.dirs, &spec.data_dir, cmd.acc_conf, &passwords)?);
	let rpc_stats = Arc::new(informant::RpcStats::default());

	// the dapps server
	let signer_service = Arc::new(signer::new_service(&cmd.ws_conf, &cmd.logger_config));

	// start RPCs
	let deps_for_rpc_apis = Arc::new(rpc_apis::LightDependencies {
		signer_service,
		client: client.clone(),
		sync: light_sync.clone(),
		net: light_sync.clone(),
		accounts: account_provider,
		logger,
		settings: Arc::new(cmd.net_settings),
		on_demand,
		cache: cache.clone(),
		transaction_queue: txq,
		ws_address: cmd.ws_conf.address(),
		fetch,
		geth_compatibility: cmd.geth_compatibility,
		experimental_rpcs: cmd.experimental_rpcs,
		executor: runtime.executor(),
		private_tx_service: None, //TODO: add this to client.
		gas_price_percentile: cmd.gas_price_percentile,
		poll_lifetime: cmd.poll_lifetime
	});

	let dependencies = rpc::Dependencies {
		apis: deps_for_rpc_apis.clone(),
		executor: runtime.executor(),
		stats: rpc_stats.clone(),
	};

	// start rpc servers
	let rpc_direct = rpc::setup_apis(rpc_apis::ApiSet::All, &dependencies);
	let ws_server = rpc::new_ws(cmd.ws_conf, &dependencies)?;
	let http_server = rpc::new_http("HTTP JSON-RPC", "jsonrpc", cmd.http_conf.clone(), &dependencies)?;
	let ipc_server = rpc::new_ipc(cmd.ipc_conf, &dependencies)?;

	// the informant
	let informant = Arc::new(Informant::new(
		LightNodeInformantData {
			client: client.clone(),
			sync: light_sync.clone(),
			cache,
		},
		None,
		Some(rpc_stats),
		cmd.logger_config.color,
	));
	service.add_notify(informant.clone());
	service.register_handler(informant.clone()).map_err(|_| "Unable to register informant handler".to_owned())?;

	client.set_exit_handler(on_client_rq);

	Ok(RunningClient {
		inner: RunningClientInner::Light {
			rpc: rpc_direct,
			informant,
			client,
			keep_alive: Box::new((service, ws_server, http_server, ipc_server, runtime)),
		}
	})
}

/// Parity client currently executing in background threads.
///
/// Should be destroyed by calling `shutdown()`, otherwise execution will continue in the
/// background.
pub struct RunningClient {
	inner: RunningClientInner,
}

enum RunningClientInner {
	Light {
		rpc: jsonrpc_core::MetaIoHandler<Metadata, informant::Middleware<rpc_apis::LightClientNotifier>>,
		informant: Arc<Informant<LightNodeInformantData>>,
		client: Arc<LightClient>,
		keep_alive: Box<dyn Any>,
	},
//	Full {
//		rpc: jsonrpc_core::MetaIoHandler<Metadata, informant::Middleware<informant::ClientNotifier>>,
//		informant: Arc<Informant<FullNodeInformantData>>,
//		client: Arc<Client>,
//		client_service: Arc<ClientService>,
//		keep_alive: Box<dyn Any>,
//	},
}

impl RunningClient {
	/// Performs an asynchronous RPC query.
	// FIXME: [tomaka] This API should be better, with for example a Future
	pub fn rpc_query(&self, request: &str, session: Option<Arc<PubSubSession>>)
		-> FutureResult<FutureResponse, FutureOutput>
	{
		let metadata = Metadata {
			origin: Origin::CApi,
			session,
		};

		match self.inner {
			RunningClientInner::Light { ref rpc, .. } => rpc.handle_request(request, metadata),
//			RunningClientInner::Full { ref rpc, .. } => rpc.handle_request(request, metadata),
		}
	}

	/// Shuts down the client.
	pub fn shutdown(self) {
		match self.inner {
			RunningClientInner::Light { rpc, informant, client, keep_alive } => {
				// Create a weak reference to the client so that we can wait on shutdown
				// until it is dropped
				let weak_client = Arc::downgrade(&client);
				drop(rpc);
				drop(keep_alive);
				informant.shutdown();
				drop(informant);
				drop(client);
				wait_for_drop(weak_client);
			},
//			RunningClientInner::Full { rpc, informant, client, client_service, keep_alive } => {
//				info!("Finishing work, please wait...");
//				// Create a weak reference to the client so that we can wait on shutdown
//				// until it is dropped
//				let weak_client = Arc::downgrade(&client);
//				// Shutdown and drop the ClientService
//				client_service.shutdown();
//				trace!(target: "shutdown", "ClientService shut down");
//				drop(client_service);
//				trace!(target: "shutdown", "ClientService dropped");
//				// drop this stuff as soon as exit detected.
//				drop(rpc);
//				trace!(target: "shutdown", "RPC dropped");
//				drop(keep_alive);
//				trace!(target: "shutdown", "KeepAlive dropped");
//				// to make sure timer does not spawn requests while shutdown is in progress
//				informant.shutdown();
//				trace!(target: "shutdown", "Informant shut down");
//				// just Arc is dropping here, to allow other reference release in its default time
//				drop(informant);
//				trace!(target: "shutdown", "Informant dropped");
//				drop(client);
//				trace!(target: "shutdown", "Client dropped");
//				// This may help when debugging ref cycles. Requires nightly-only  `#![feature(weak_counts)]`
//				// trace!(target: "shutdown", "Waiting for refs to Client to shutdown, strong_count={:?}, weak_count={:?}", weak_client.strong_count(), weak_client.weak_count());
//				trace!(target: "shutdown", "Waiting for refs to Client to shutdown");
//				wait_for_drop(weak_client);
//			}
		}
	}
}

/// Executes the given run command.
///
/// `on_client_rq` is the action to perform when the client receives an RPC request to be restarted
/// with a different chain.
///
/// `on_updater_rq` is the action to perform when the updater has a new binary to execute.
///
/// On error, returns what to print on stderr.
pub fn execute<Cr, Rr>(cmd: RunCmd, logger: Arc<RotatingLogger>,
						on_client_rq: Cr, on_updater_rq: Rr) -> Result<RunningClient, String>
	where Cr: Fn(String) + 'static + Send,
		Rr: Fn() + 'static + Send
{
		execute_light_impl(cmd, logger, on_client_rq)
}

fn print_running_environment(data_dir: &str, dirs: &Directories, db_dirs: &DatabaseDirectories) {
	info!("Starting {}", Colour::White.bold().paint(version()));
	info!("Keys path {}", Colour::White.bold().paint(dirs.keys_path(data_dir).to_string_lossy().into_owned()));
	info!("DB path {}", Colour::White.bold().paint(db_dirs.db_root_path().to_string_lossy().into_owned()));
}

fn wait_for_drop<T>(w: Weak<T>) {
	const SLEEP_DURATION: Duration = Duration::from_secs(1);
	const WARN_TIMEOUT: Duration = Duration::from_secs(60);
	const MAX_TIMEOUT: Duration = Duration::from_secs(300);

	let instant = Instant::now();
	let mut warned = false;

	while instant.elapsed() < MAX_TIMEOUT {
		if w.upgrade().is_none() {
			return;
		}

		if !warned && instant.elapsed() > WARN_TIMEOUT {
			warned = true;
			warn!("Shutdown is taking longer than expected.");
		}

		thread::sleep(SLEEP_DURATION);

		// When debugging shutdown issues on a nightly build it can help to enable this with the
		// `#![feature(weak_counts)]` added to lib.rs (TODO: enable when
		// https://github.com/rust-lang/rust/issues/57977 is stable)
		// trace!(target: "shutdown", "Waiting for client to drop, strong_count={:?}, weak_count={:?}", w.strong_count(), w.weak_count());
		trace!(target: "shutdown", "Waiting for client to drop");
	}

	warn!("Shutdown timeout reached, exiting uncleanly.");
}

