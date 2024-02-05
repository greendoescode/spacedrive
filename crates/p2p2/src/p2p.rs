use std::{
	collections::{hash_map::Entry, HashMap, HashSet},
	net::SocketAddr,
	sync::{atomic::AtomicUsize, Arc, PoisonError, RwLock, RwLockReadGuard},
};

use hash_map_diff::hash_map_diff;
use stable_vec::StableVec;
use tokio::sync::{mpsc, oneshot};

use crate::{
	hooks::{HandlerFn, Hook, HookEvent, ListenerData, ListenerId},
	smart_guards::SmartWriteGuard,
	HookId, Identity, Peer, RemoteIdentity, UnicastStream,
};

/// Manager for the entire P2P system.
#[derive(Debug)]
pub struct P2P {
	/// A unique identifier for the application.
	/// This will differentiate between different applications using this same P2P library.
	app_name: &'static str,
	/// The identity of the local node.
	/// This is the public/private keypair used to uniquely identify the node.
	identity: Identity,
	/// The channel is used by the application to handle incoming connections.
	/// Connection's are automatically closed when dropped so if user forgets to subscribe to this that will just happen as expected.
	handler_tx: mpsc::Sender<UnicastStream>,
	/// Metadata is shared from the local node to the remote nodes.
	/// This will contain information such as the node's name, version, and services we provide.
	metadata: RwLock<HashMap<String, String>>,
	/// A list of all peers known to the P2P system. Be aware a peer could be connected and/or discovered at any time.
	peers: RwLock<HashMap<RemoteIdentity, Arc<Peer>>>,
	/// A counter for getting a new unique ID for the next hook.
	/// This number is converted to a `HookId` and returned to the user. // TODO: Fix this
	hook_id: AtomicUsize,
	/// Hooks can be registered to react to state changes in the P2P system.
	pub(crate) hooks: RwLock<StableVec<Hook>>,
}

impl P2P {
	/// Construct a new P2P system.
	pub fn new(
		app_name: &'static str,
		identity: Identity,
		handler_tx: mpsc::Sender<UnicastStream>,
	) -> Arc<Self> {
		app_name
			.chars()
			.all(|c| char::is_alphanumeric(c) || c == '-')
			.then_some(())
			.expect("'P2P::new': invalid app_name. Must be alphanumeric or '-' only.");
		if app_name.len() > 12 {
			panic!("'P2P::new': app_name too long. Must be 12 characters or less.");
		}

		Arc::new(P2P {
			app_name,
			identity,
			metadata: Default::default(),
			peers: Default::default(),
			handler_tx,
			hook_id: Default::default(),
			hooks: Default::default(),
		})
	}

	/// The unique identifier for this application.
	pub fn app_name(&self) -> &'static str {
		self.app_name
	}

	/// The identifier of this node that can *MUST* be kept secret.
	/// This is a private key in crypto terms.
	pub fn identity(&self) -> &Identity {
		&self.identity
	}

	/// The identifier of this node that can be shared.
	/// This is a public key in crypto terms.
	pub fn remote_identity(&self) -> RemoteIdentity {
		self.identity.to_remote_identity()
	}

	/// Metadata is shared from the local node to the remote nodes.
	/// This will contain information such as the node's name, version, and services we provide.
	pub fn metadata(&self) -> RwLockReadGuard<HashMap<String, String>> {
		self.metadata.read().unwrap_or_else(PoisonError::into_inner)
	}

	pub fn metadata_mut(&self) -> SmartWriteGuard<HashMap<String, String>> {
		let lock = self
			.metadata
			.write()
			.unwrap_or_else(PoisonError::into_inner);

		SmartWriteGuard::new(self, lock, |p2p, before, after| {
			let diff = hash_map_diff(&before, after);
			if diff.updated.is_empty() && diff.removed.is_empty() {
				return;
			}

			p2p.hooks
				.read()
				.unwrap_or_else(PoisonError::into_inner)
				.iter()
				.for_each(|(id, hook)| hook.send(HookEvent::MetadataModified));
		})
	}

	/// A list of all peers known to the P2P system. Be aware a peer could be connected and/or discovered at any time.
	pub fn peers(&self) -> RwLockReadGuard<HashMap<RemoteIdentity, Arc<Peer>>> {
		self.peers.read().unwrap_or_else(PoisonError::into_inner)
	}

	// TODO: Should this take `addrs`???, A connection through the Relay probs doesn't have one in the same form.
	pub fn discover_peer(
		self: Arc<Self>,
		hook_id: HookId,
		identity: RemoteIdentity,
		metadata: HashMap<String, String>,
		addrs: Vec<SocketAddr>,
	) -> Arc<Peer> {
		// let mut peer = self
		// 	.peers
		// 	.write()
		// 	.unwrap_or_else(PoisonError::into_inner)
		// 	.entry(identity);
		// let was_peer_inserted = matches!(peer, Entry::Vacant(_));
		// let peer = peer
		// 	.or_insert_with({
		// 		let p2p = self.clone();
		// 		|| Peer::new(identity, p2p)
		// 	})
		// 	.clone();

		// {
		// 	let mut state = peer.state.write().unwrap_or_else(PoisonError::into_inner);
		// 	state.discovered.insert(hook_id);
		// }

		// peer.metadata_mut().extend(metadata);

		// {
		// 	let hooks = self.hooks.read().unwrap_or_else(PoisonError::into_inner);
		// 	hooks
		// 		.iter()
		// 		.for_each(|(id, hook)| hook.acceptor(&peer, &addrs));

		// 	if was_peer_inserted {
		// 		hooks
		// 			.iter()
		// 			.for_each(|(id, hook)| hook.send(HookEvent::PeerAvailable(peer.clone())));
		// 	}

		// 	hooks.iter().for_each(|(id, hook)| {
		// 		hook.send(HookEvent::PeerDiscoveredBy(hook_id, peer.clone()))
		// 	});
		// }

		// peer
		todo!();
	}

	pub fn connected_to(
		self: Arc<Self>,
		listener: ListenerId,
		identity: RemoteIdentity,
		metadata: HashMap<String, String>,
		shutdown_tx: oneshot::Sender<()>,
	) -> Arc<Peer> {
		// let mut peer = self
		// 	.peers
		// 	.write()
		// 	.unwrap_or_else(PoisonError::into_inner)
		// 	.entry(identity);
		// let was_peer_inserted = matches!(peer, Entry::Vacant(_));
		// let peer = peer
		// 	.or_insert_with({
		// 		let p2p = self.clone();
		// 		move || Peer::new(identity, p2p)
		// 	})
		// 	.clone();

		// {
		// 	let mut state = peer.state.write().unwrap_or_else(PoisonError::into_inner);
		// 	state.active_connections.insert(listener, shutdown_tx);
		// }

		// peer.metadata_mut().extend(metadata);

		// {
		// 	let hooks = self.hooks.read().unwrap_or_else(PoisonError::into_inner);

		// 	if was_peer_inserted {
		// 		hooks
		// 			.iter()
		// 			.for_each(|(id, hook)| hook.send(HookEvent::PeerAvailable(peer.clone())));
		// 	}

		// 	// hooks.iter().for_each(|(id, hook)| {
		// 	// 	hook.send(HookEvent::PeerConnectedWith(listener, peer.clone()))
		// 	// });
		// 	todo!();
		// }

		// peer
		todo!();
	}

	/// All active listeners registered with the P2P system.
	pub fn listeners(&self) -> Vec<Listener> {
		// self.hooks
		// 	.read()
		// 	.unwrap_or_else(PoisonError::into_inner)
		// 	.iter()
		// 	.filter_map(|(id, hook)| {
		// 		hook.listener.map(|listener| Listener {
		// 			id: ListenerId(id),
		// 			name: hook.name,
		// 			addrs: listener.addrs,
		// 		})
		// 	})
		// 	.collect()

		todo!();
	}

	/// A listener is a special type of hook which is responsible for accepting incoming connections.
	///
	/// It is expected you call `Self::register_listener_addr` after this to register the addresses you are listening on.
	pub fn register_listener(
		&self,
		name: &'static str,
		tx: mpsc::Sender<HookEvent>,
		acceptor: impl Fn(&Arc<Peer>, &Vec<SocketAddr>) + Send + Sync + 'static,
	) -> ListenerId {
		let id = self
			.hooks
			.write()
			.unwrap_or_else(PoisonError::into_inner)
			.push(Hook {
				name,
				tx,
				listener: Some(ListenerData {
					addrs: Default::default(),
					acceptor: HandlerFn(Arc::new(acceptor)),
				}),
			});

		ListenerId(id)
	}

	pub fn register_listener_addr(&self, listener_id: ListenerId, addr: SocketAddr) {
		self.hooks
			.read()
			.unwrap_or_else(PoisonError::into_inner)
			.iter()
			.for_each(|(id, hook)| {
				hook.send(HookEvent::ListenerRegistered {
					id: ListenerId(id),
					addr,
				});
			});
	}

	pub fn unregister_listener_addr(&self, listener_id: ListenerId, addr: SocketAddr) {
		self.hooks
			.read()
			.unwrap_or_else(PoisonError::into_inner)
			.iter()
			.for_each(|(id, hook)| {
				hook.send(HookEvent::ListenerUnregistered(ListenerId(id)));
			});
	}

	/// Register a new hook which can be used to react to state changes in the P2P system.
	pub fn register_hook(&self, name: &'static str, tx: mpsc::Sender<HookEvent>) -> HookId {
		HookId(
			self.hooks
				.write()
				.unwrap_or_else(PoisonError::into_inner)
				.push(Hook {
					name,
					tx,
					listener: None,
				}),
		)
	}

	/// Unregister a hook. This will also call `HookEvent::Shutdown` on the hook.
	pub fn unregister_hook(&self, id: HookId) {
		if let Some(hook) = self
			.hooks
			.write()
			.unwrap_or_else(PoisonError::into_inner)
			.remove(id.0)
		{
			let _ = hook.send(HookEvent::Shutdown);

			if let Some(_) = hook.listener {
				self.hooks
					.read()
					.unwrap_or_else(PoisonError::into_inner)
					.iter()
					.for_each(|(id, hook)| {
						hook.send(HookEvent::ListenerUnregistered(ListenerId(id)))
					});
			}

			let mut peers = self.peers.write().unwrap_or_else(PoisonError::into_inner);
			for (identity, peer) in peers.iter_mut() {
				let mut state = peer.state.write().unwrap_or_else(PoisonError::into_inner);
				if let Some(active_connection) = state.active_connections.remove(&ListenerId(id.0))
				{
					let _ = active_connection.send(());
				}
				state.connection_methods.remove(&ListenerId(id.0));
				state.discovered.remove(&id);

				if state.connection_methods.is_empty() && state.discovered.is_empty() {
					// peers.remove(&identity);
					todo!(); // TODO: mutating in iteration is bad, thanks compiler
				}
			}
		}
	}

	/// Shutdown the whole P2P system.
	/// This will close all connections and remove all hooks.
	pub fn shutdown(&self) {
		let mut hooks = self
			.hooks
			.write()
			.unwrap_or_else(PoisonError::into_inner)
			.iter()
			.map(|i| i.0)
			.collect::<Vec<_>>();

		for hook_id in hooks {
			self.unregister_hook(HookId(hook_id));
		}

		// TODO: Wait for response from hooks saying they are done shutting down
		// TODO: Maybe wait until `unregister_hook` is called internally by each of them or with timeout and force removal overwise.
	}
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Listener {
	pub id: ListenerId,
	pub name: &'static str,
	pub addrs: HashSet<SocketAddr>,
}
