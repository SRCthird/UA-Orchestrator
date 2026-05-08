use opcua_client::prelude::*;
use std::sync::{RwLock, Arc};
use gethostname::gethostname;
use whoami::username;
use crate::config::Config;
use crate::globals::Globals;


pub trait OpcUaClient {
    fn read(&self, node_id: &NodeId) -> Option<Variant>;
    fn write(&mut self, node_id: &NodeId, value: Variant);
}

pub struct LiveOpcUaClient<'a> {
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

pub trait SessionBackend {
    fn read_nodes(&self, nodes: &[ReadValueId]) -> Result<Vec<DataValue>, StatusCode>;
    fn write_nodes(&self, nodes: &[WriteValue])  -> Result<Vec<StatusCode>, StatusCode>;
}

pub struct LiveSessionBackend {
    session: Arc<RwLock<Session>>,
}

impl LiveSessionBackend {
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

pub struct OpcUaSession<B: SessionBackend = LiveSessionBackend> {
    backend: B,
}

impl OpcUaSession<LiveSessionBackend> {
    pub fn new(config: &Config) -> Self {
        let session = Self::create_session(config);
        Self {
            backend: LiveSessionBackend::new(session),
        }
    }

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
    #[allow(unused)]
    pub fn with_backend(backend: B) -> Self {
        Self { backend }
    }

    fn make_read_value_id(node_id: &NodeId) -> ReadValueId {
        ReadValueId {
            node_id:       node_id.clone(),
            attribute_id:  AttributeId::Value as u32,
            index_range:   UAString::null(),
            data_encoding: QualifiedName::null(),
        }
    }

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

    pub fn read(&self, node_id: &NodeId) -> Option<Variant> {
        self.read_many(&[node_id.clone()])
            .into_iter()
            .next()
            .flatten()
    }

    pub fn write(&self, node_id: &NodeId, value: Variant) {
        self.write_many(&[(node_id.clone(), value)]);
    }

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

    pub struct FakeSessionBackend {
        pub store: Mutex<HashMap<NodeId, Variant>>,
    }

    impl FakeSessionBackend {
        pub fn new() -> Self {
            Self { store: Mutex::new(HashMap::new()) }
        }

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

