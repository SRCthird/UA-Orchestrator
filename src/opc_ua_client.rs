// // Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// // All rights reserved

//! # OPC UA Client
//!
//! This module provides the full OPC UA communication stack for UAOrchestrator,
//! structured in two independent abstraction layers so that every public
//! interface can be exercised in unit tests without a real OPC UA server.
//!
//! ## Architecture
//!
//! ```text
//!  actions / main
//!  (uses OpcUaClient trait only)
//!        |
//!        | OpcUaClient
//!        v
//!  LiveOpcUaClient       thin adapter holding a &OpcUaSession
//!        |
//!        | delegates to
//!        v
//!  OpcUaSession<B>       read / write / read_many / write_many
//!        |
//!        | SessionBackend
//!        +-----------------------------+
//!        |                             |
//!  LiveSessionBackend        FakeSessionBackend  (test only)
//!  (Arc<RwLock<Session>>)    (Mutex<HashMap>)
//! ```
//!
//! ## Layer 1 — [`OpcUaClient`] trait
//!
//! The interface consumed by [`actions::run_csv`](crate::actions::run_csv).
//! Exposes only [`read`](OpcUaClient::read) and [`write`](OpcUaClient::write)
//! for single nodes, keeping the action dispatcher decoupled from session
//! internals.
//!
//! ## Layer 2 — [`SessionBackend`] trait
//!
//! The interface consumed by [`OpcUaSession`]. Wraps the raw OPC UA service
//! calls ([`read_nodes`](SessionBackend::read_nodes) /
//! [`write_nodes`](SessionBackend::write_nodes)) so that
//! [`FakeSessionBackend`](tests::FakeSessionBackend) can replace them in tests
//! without any network interaction.
//!
//! ## Typical Production Call Chain
//!
//! ```text
//! Config::load(...)                       reads Config.toml
//!   => OpcUaSession::new(&config)         connects and authenticates
//!     => LiveOpcUaClient { session }      borrows the live session
//!       => actions::run_csv(...)          executes the CSV script
//! ```
//!
//! ## Panics
//!
//! [`OpcUaSession::new`] panics if:
//!
//! - The server is unreachable or rejects the connection.
//! - No endpoint matches the configured security policy and mode.
//! - Authentication fails.

use opcua_client::prelude::*;
use std::sync::{RwLock, Arc};
use gethostname::gethostname;
use whoami::username;
use crate::config::Config;
use crate::globals::Globals;

// -----------------------------------------------------------------------------
// OpcUaClient trait
// -----------------------------------------------------------------------------

/// High-level OPC UA client interface consumed by the CSV action dispatcher.
///
/// Abstracting over this trait allows [`crate::actions`] to be tested without
/// a live OPC UA server by substituting a fake implementation.
///
/// # Implementors
///
/// | Type | Purpose |
/// |---|---|
/// | [`LiveOpcUaClient`] | Production — delegates to [`OpcUaSession`] |
/// | `FakeClient` (tests) | In-memory store used by `actions` unit tests |
pub trait OpcUaClient {
    /// Reads the current `Value` attribute of a single OPC UA node.
    ///
    /// Returns `Some(variant)` on success, or `None` if the node has no
    /// data value or the read fails.
    ///
    /// # Arguments
    /// * `node_id` — The [`NodeId`] of the node to read.
    fn read(&self, node_id: &NodeId) -> Option<Variant>;

    /// Writes a value to the `Value` attribute of a single OPC UA node.
    ///
    /// Errors are logged to `stderr` via [`Globals::write_error`] and
    /// silently swallowed — no `Result` is returned.
    ///
    /// # Arguments
    /// * `node_id` — The [`NodeId`] of the node to write.
    /// * `value`   — The [`Variant`] to write.
    fn write(&mut self, node_id: &NodeId, value: Variant);
}

// -----------------------------------------------------------------------------
// LiveOpcUaClient
// -----------------------------------------------------------------------------

/// Production implementation of [`OpcUaClient`] that delegates all operations
/// to an [`OpcUaSession`].
///
/// The lifetime parameter `'a` ties this client to the session it borrows,
/// ensuring the session outlives any active client reference.
///
/// # Example
///
/// ```rust,ignore
/// let session = OpcUaSession::new(&config);
/// let mut client = LiveOpcUaClient { session: &session };
/// actions::run_csv(&mut client, &mut reader, "script.csv");
/// ```
pub struct LiveOpcUaClient<'a> {
    /// Reference to the underlying OPC UA session.
    pub session: &'a OpcUaSession,
}

impl<'a> OpcUaClient for LiveOpcUaClient<'a> {
    fn read(&self, node_id: &NodeId) -> Option<Variant> {
        self.session.read(node_id)
    }

    fn write(&mut self, node_id: &NodeId, value: Variant) {
        self.session.write(node_id, value);
    }
}

// -----------------------------------------------------------------------------
// SessionBackend trait
// -----------------------------------------------------------------------------

/// Low-level OPC UA service interface used internally by [`OpcUaSession`].
///
/// This trait isolates the raw OPC UA service calls behind a seam, enabling
/// [`FakeSessionBackend`](tests::FakeSessionBackend) to replace the real
/// network stack in unit tests.
///
/// # Implementors
///
/// | Type | Purpose |
/// |---|---|
/// | [`LiveSessionBackend`] | Production — wraps `Arc<RwLock<Session>>` |
/// | [`tests::FakeSessionBackend`] | Test double — `Mutex<HashMap>` store |
pub trait SessionBackend {
    /// Sends a batch read request to the OPC UA server.
    ///
    /// # Arguments
    /// * `nodes` — Slice of [`ReadValueId`] descriptors (one per node/attribute).
    ///
    /// # Returns
    /// `Ok(Vec<DataValue>)` in the same order as `nodes`, or
    /// `Err(StatusCode)` if the service call itself fails.
    fn read_nodes(&self, nodes: &[ReadValueId]) -> Result<Vec<DataValue>, StatusCode>;

    /// Sends a batch write request to the OPC UA server.
    ///
    /// # Arguments
    /// * `nodes` — Slice of [`WriteValue`] descriptors (one per node/attribute).
    ///
    /// # Returns
    /// `Ok(Vec<StatusCode>)` with a per-node result in the same order as
    /// `nodes`, or `Err(StatusCode)` if the service call itself fails.
    fn write_nodes(&self, nodes: &[WriteValue])  -> Result<Vec<StatusCode>, StatusCode>;
}

// -----------------------------------------------------------------------------
// LiveSessionBackend
// -----------------------------------------------------------------------------

/// Production [`SessionBackend`] that forwards calls to an
/// `Arc<RwLock<Session>>`.
///
/// The `Arc<RwLock<_>>` wrapper is required by `opcua_client`, which returns
/// the session in that form from [`ClientBuilder`].
pub struct LiveSessionBackend {
    session: Arc<RwLock<Session>>,
}

impl LiveSessionBackend {
    /// Wraps an existing `Arc<RwLock<Session>>` in a [`LiveSessionBackend`].
    ///
    /// # Arguments
    /// * `session` — A shared, lock-guarded OPC UA session handle as returned
    ///               by [`ClientBuilder`].
    pub fn new(session: Arc<RwLock<Session>>) -> Self {
        Self { session }
    }
}

impl SessionBackend for LiveSessionBackend {
    fn read_nodes(&self, nodes: &[ReadValueId]) -> Result<Vec<DataValue>, StatusCode> {
        self.session
            .read()
            .unwrap()
            .read(nodes, TimestampsToReturn::Both, 0.0)
    }

    fn write_nodes(&self, nodes: &[WriteValue]) -> Result<Vec<StatusCode>, StatusCode> {
        self.session.read().unwrap().write(nodes)
    }
}

// -----------------------------------------------------------------------------
// OpcUaSession
// -----------------------------------------------------------------------------

/// Session manager that provides single-node and batch OPC UA read/write
/// operations on top of a [`SessionBackend`].
///
/// The type parameter `B` defaults to [`LiveSessionBackend`] in production.
/// Tests substitute [`FakeSessionBackend`](tests::FakeSessionBackend) via
/// [`OpcUaSession::with_backend`].
///
/// # Type Parameters
///
/// * `B` — Any type that implements [`SessionBackend`].
pub struct OpcUaSession<B: SessionBackend = LiveSessionBackend> {
    backend: B,
}

impl OpcUaSession<LiveSessionBackend> {
    /// Creates a new [`OpcUaSession`] connected to the OPC UA server described
    /// by `config`.
    ///
    /// Internally calls [`OpcUaSession::create_session`] which:
    ///
    /// 1. Resolves the hostname and OS username.
    /// 2. Prints startup banner lines via [`Globals`].
    /// 3. Builds an `opcua_client` [`ClientBuilder`] with per-user PKI paths.
    /// 4. Discovers server endpoints and selects the one matching the
    ///    configured security policy and mode.
    /// 5. Connects using [`IdentityToken::UserName`] credentials.
    ///
    /// # Arguments
    /// * `config` — Application configuration loaded from `Config.toml`.
    ///
    /// # Panics
    ///
    /// Panics if:
    ///
    /// - The server URL is unreachable.
    /// - No endpoint matches the configured security policy and mode.
    /// - Authentication is rejected by the server.
    pub fn new(config: &Config) -> Self {
        let session = Self::create_session(config);
        Self {
            backend: LiveSessionBackend::new(session),
        }
    }

    /// Discovers endpoints, selects the correct one, and establishes an
    /// authenticated OPC UA session.
    ///
    /// This is a private helper called exclusively by [`OpcUaSession::new`].
    /// It is responsible for all network I/O performed at start-up.
    fn create_session(config: &Config) -> Arc<RwLock<Session>> {
        let hostname = gethostname().into_string().unwrap();
        let user     = username().unwrap();

        let app_name = Globals::app_name(&user);
        let app_uri  = Globals::app_uri(&hostname);

        println!("{}", Globals::app_user_msg(&user));
        println!("{}", Globals::app_name_msg(&app_name));
        println!("{}", Globals::app_uri_msg(&app_uri));

        let mut client = ClientBuilder::new()
            .application_name(app_name)
            .application_uri(app_uri)
            .certificate_path(Globals::user_cert(&user))
            .private_key_path(Globals::user_key(&user))
            .create_sample_keypair(true)
            .trust_server_certs(true)
            .client()
            .unwrap();

        let security_policy = config.security_policy();
        let security_mode   = config.security_mode();

        let endpoint = client
            .get_server_endpoints_from_url(&config.server_url)
            .unwrap()
            .into_iter()
            .find(|e| {
                e.security_policy_uri.as_ref() == security_policy.to_uri()
                    && e.security_mode == security_mode
            })
            .expect(Globals::endpoint_error());

        println!("{}", Globals::endpoint_connecting(endpoint.endpoint_url.as_ref()));

        client
            .connect_to_endpoint(
                endpoint,
                IdentityToken::UserName(
                    config.username.to_string(),
                    config.password.to_string(),
                ),
            )
            .unwrap()
    }
}

impl<B: SessionBackend> OpcUaSession<B> {
    /// Creates an [`OpcUaSession`] backed by a custom [`SessionBackend`].
    ///
    /// Intended for unit testing. Pass a
    /// [`FakeSessionBackend`](tests::FakeSessionBackend) to exercise session
    /// logic without a real server.
    ///
    /// # Arguments
    /// * `backend` — Any [`SessionBackend`] implementation.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let session = OpcUaSession::with_backend(FakeSessionBackend::new());
    /// ```
    #[allow(unused)]
    pub fn with_backend(backend: B) -> Self {
        Self { backend }
    }

    /// Constructs a [`ReadValueId`] targeting the `Value` attribute of a node.
    fn make_read_value_id(node_id: &NodeId) -> ReadValueId {
        ReadValueId {
            node_id:       node_id.clone(),
            attribute_id:  AttributeId::Value as u32,
            index_range:   UAString::null(),
            data_encoding: QualifiedName::null(),
        }
    }

    /// Constructs a [`WriteValue`] for writing a [`Variant`] to the `Value`
    /// attribute of a node.
    fn make_write_value(node_id: &NodeId, value: Variant) -> WriteValue {
        WriteValue {
            node_id:      node_id.clone(),
            attribute_id: AttributeId::Value as u32,
            index_range:  UAString::null(),
            value: DataValue {
                value:              Some(value),
                status:             None,
                source_timestamp:   None,
                source_picoseconds: None,
                server_timestamp:   None,
                server_picoseconds: None,
            },
        }
    }

    /// Reads the `Value` attribute of a single OPC UA node.
    ///
    /// A convenience wrapper around [`read_many`](Self::read_many) for the
    /// common single-node case.
    ///
    /// # Arguments
    /// * `node_id` — The [`NodeId`] to read.
    ///
    /// # Returns
    /// `Some(variant)` if the node has a value, `None` otherwise.
    pub fn read(&self, node_id: &NodeId) -> Option<Variant> {
        self.read_many(&[node_id.clone()])
            .into_iter()
            .next()
            .flatten()
    }

    /// Writes a [`Variant`] to the `Value` attribute of a single OPC UA node.
    ///
    /// A convenience wrapper around [`write_many`](Self::write_many) for the
    /// common single-node case.
    ///
    /// # Arguments
    /// * `node_id` — The [`NodeId`] to write.
    /// * `value`   — The [`Variant`] to write.
    pub fn write(&self, node_id: &NodeId, value: Variant) {
        self.write_many(&[(node_id.clone(), value)]);
    }

    /// Reads the `Value` attribute of multiple OPC UA nodes in a single
    /// service call.
    ///
    /// Results are returned in the **same order** as `node_ids`. Missing or
    /// unreadable nodes produce `None` entries; a warning is printed for each
    /// via [`Globals::read_no_value`]. If the entire service call fails, an
    /// error is printed via [`Globals::read_error`] and a `vec![None; n]` is
    /// returned.
    ///
    /// # Arguments
    /// * `node_ids` — Slice of [`NodeId`] values to read.
    ///
    /// # Returns
    /// A `Vec<Option<Variant>>` of the same length as `node_ids`.
    pub fn read_many(&self, node_ids: &[NodeId]) -> Vec<Option<Variant>> {
        let nodes_to_read: Vec<ReadValueId> = node_ids
            .iter()
            .map(Self::make_read_value_id)
            .collect();

        match self.backend.read_nodes(&nodes_to_read) {
            Ok(results) => results
                .into_iter()
                .enumerate()
                .map(|(i, dv)| match dv.value {
                    Some(v) => Some(v),
                    None => {
                        println!(
                            "{}",
                            Globals::read_no_value(i, dv.status.unwrap_or(StatusCode::BadNoData))
                        );
                        None
                    }
                })
                .collect(),
            Err(e) => {
                eprintln!("{}", Globals::read_error(e));
                vec![None; node_ids.len()]
            }
        }
    }

    /// Writes [`Variant`] values to multiple OPC UA nodes in a single service
    /// call.
    ///
    /// Per-node write failures are currently ignored (the commented-out
    /// [`Globals::write_status`] call is intentionally disabled). Service-level
    /// failures are printed to `stderr` via [`Globals::write_error`].
    ///
    /// # Arguments
    /// * `pairs` — Slice of `(NodeId, Variant)` tuples. Each tuple specifies
    ///             the target node and the value to write.
    pub fn write_many(&self, pairs: &[(NodeId, Variant)]) {
        let nodes_to_write: Vec<WriteValue> = pairs
            .iter()
            .map(|(id, val)| Self::make_write_value(id, val.clone()))
            .collect();

        match self.backend.write_nodes(&nodes_to_write) {
            Ok(results) => {
                for (_i, _status) in results.iter().enumerate() {
                    // println!("{}", Globals::write_status(i, status));
                }
            }
            Err(e) => eprintln!("{}", Globals::write_error(e)),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory [`SessionBackend`] used by unit tests.
    ///
    /// Maintains a `Mutex<HashMap<NodeId, Variant>>` store that is read and
    /// written synchronously. Pre-populate it with [`FakeSessionBackend::seed`]
    /// before passing it to [`OpcUaSession::with_backend`].
    pub struct FakeSessionBackend {
        /// Shared node value store. Publicly accessible for test assertions.
        pub store: Mutex<HashMap<NodeId, Variant>>,
    }

    impl FakeSessionBackend {
        /// Creates an empty [`FakeSessionBackend`].
        pub fn new() -> Self {
            Self { store: Mutex::new(HashMap::new()) }
        }

        /// Seeds the store with an initial `(NodeId, Variant)` entry and
        /// returns `self` for chaining.
        ///
        /// # Example
        ///
        /// ```rust,ignore
        /// let backend = FakeSessionBackend::new()
        ///     .seed(NodeId::new(2, "A"), Variant::Int32(1))
        ///     .seed(NodeId::new(2, "B"), Variant::Int32(2));
        /// ```
        pub fn seed(self, node_id: NodeId, value: Variant) -> Self {
            self.store.lock().unwrap().insert(node_id, value);
            self
        }
    }

    impl SessionBackend for FakeSessionBackend {
        fn read_nodes(&self, nodes: &[ReadValueId]) -> Result<Vec<DataValue>, StatusCode> {
            let store = self.store.lock().unwrap();
            let results = nodes
                .iter()
                .map(|rvid| DataValue {
                    value: store.get(&rvid.node_id).cloned(),
                    status:             None,
                    source_timestamp:   None,
                    source_picoseconds: None,
                    server_timestamp:   None,
                    server_picoseconds: None,
                })
                .collect();
            Ok(results)
        }

        fn write_nodes(&self, nodes: &[WriteValue]) -> Result<Vec<StatusCode>, StatusCode> {
            let mut store = self.store.lock().unwrap();
            let statuses = nodes
                .iter()
                .map(|wv| {
                    if let Some(v) = wv.value.value.clone() {
                        store.insert(wv.node_id.clone(), v);
                        StatusCode::Good
                    } else {
                        StatusCode::BadNoData
                    }
                })
                .collect();
            Ok(statuses)
        }
    }

    fn fake_session(node_id: &NodeId, value: Variant) -> OpcUaSession<FakeSessionBackend> {
        OpcUaSession::with_backend(
            FakeSessionBackend::new().seed(node_id.clone(), value)
        )
    }

    #[test]
    fn read_returns_seeded_value() {
        let node_id = NodeId::new(2, "TestNode");
        let session = fake_session(&node_id, Variant::Boolean(true));
        assert_eq!(session.read(&node_id), Some(Variant::Boolean(true)));
    }

    #[test]
    fn read_returns_none_for_missing_node() {
        let session = OpcUaSession::with_backend(FakeSessionBackend::new());
        let node_id = NodeId::new(2, "Missing");
        assert_eq!(session.read(&node_id), None);
    }

    #[test]
    fn write_then_read_roundtrip() {
        let node_id = NodeId::new(2, "RoundTrip");
        let session = OpcUaSession::with_backend(FakeSessionBackend::new());
        session.write(&node_id, Variant::Int32(42));
        assert_eq!(session.read(&node_id), Some(Variant::Int32(42)));
    }

    #[test]
    fn read_many_returns_correct_order() {
        let ids: Vec<NodeId> = (0..3).map(|i| NodeId::new(2, format!("N{i}"))).collect();
        let backend = FakeSessionBackend::new()
            .seed(ids[0].clone(), Variant::Int32(10))
            .seed(ids[2].clone(), Variant::Int32(30));
        let session = OpcUaSession::with_backend(backend);
        let results = session.read_many(&ids);
        assert_eq!(results[0], Some(Variant::Int32(10)));
        assert_eq!(results[1], None);
        assert_eq!(results[2], Some(Variant::Int32(30)));
    }
}
