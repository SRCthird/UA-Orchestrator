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
