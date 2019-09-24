use envfile::EnvFile;
use futures::stream::Stream;
use futures::Future;
use shiplift::builder::ContainerOptionsBuilder;
use shiplift::{
    ContainerOptions, Docker, LogsOptions, NetworkCreateOptions, PullOptions, RmContainerOptions,
};
use std::path::PathBuf;

pub mod bitcoin;
pub mod btsieve;
pub mod ethereum;

pub const DOCKER_NETWORK: &str = "create-comit-app";

pub struct ExposedPorts {
    pub for_client: bool,
    pub srcport: u32,
    pub env_file_key: String,
    pub env_file_value: Box<dyn Fn(u32) -> String>,
}

pub trait Image {
    const IMAGE: &'static str;
    const LOG_READY: &'static str;

    fn arguments_for_create() -> Vec<&'static str>;
    fn expose_ports() -> Vec<ExposedPorts>;
    fn new(endpoint: String) -> Self;
    fn post_start_actions(&self);
}

pub trait BlockchainImage: Image {
    type Address;
    type Amount;
    type TxId;
    type ClientError;

    fn fund(
        &self,
        address: Self::Address,
        value: Self::Amount,
    ) -> Box<dyn Future<Item = Self::TxId, Error = Self::ClientError> + Send + Sync>;
}

pub struct Node<I: Image> {
    container_id: String,
    pub node_image: I,
}

// TODO: Move all envfile stuff outside
// TODO: Probably good idea to convert into a builder
// TODO: Move free_local_port outside
impl<I: Image> Node<I> {
    pub fn start(
        envfile_path: PathBuf,
        name: &str,
    ) -> impl Future<Item = Self, Error = shiplift::errors::Error> {
        let name = name.to_string();

        let mut create_options = ContainerOptions::builder(I::IMAGE);
        create_options.name(&name);
        create_options.network_mode(DOCKER_NETWORK);
        create_options.cmd(I::arguments_for_create());

        let (to_write_in_env, client_endpoint, create_options) =
            <Node<I>>::write_env_file(&mut create_options);

        Docker::new()
            .images()
            .pull(&PullOptions::builder().image(I::IMAGE).build())
            // TODO: Pretty print progress
            .collect()
            .and_then(move |_| {
                Self::start_container(
                    envfile_path,
                    create_options,
                    client_endpoint,
                    to_write_in_env,
                )
            })
            .inspect(|node| {
                node.node_image.post_start_actions();
            })
    }

    pub fn start_with_volume(
        envfile_path: PathBuf,
        name: &str,
        volume: &str,
    ) -> impl Future<Item = Self, Error = shiplift::errors::Error> {
        let mut create_options = ContainerOptions::builder(I::IMAGE);
        create_options.name(&name);
        create_options.network_mode(DOCKER_NETWORK);
        create_options.cmd(I::arguments_for_create());
        create_options.volumes(vec![volume]);

        let (to_write_in_env, client_endpoint, create_options) =
            <Node<I>>::write_env_file(&mut create_options);

        Docker::new()
            .images()
            .pull(&PullOptions::builder().image(I::IMAGE).build())
            // TODO: Pretty print progress
            .collect()
            .and_then(move |_| {
                Self::start_container(
                    envfile_path,
                    create_options,
                    client_endpoint,
                    to_write_in_env,
                )
            })
            .inspect(|node| {
                node.node_image.post_start_actions();
            })
    }

    fn write_env_file(
        create_options: &mut ContainerOptionsBuilder,
    ) -> (Vec<(String, String)>, String, ContainerOptions) {
        let mut to_write_in_env: Vec<(String, String)> = vec![];
        let mut http_url: Option<String> = None;
        for expose_port in I::expose_ports() {
            let port: u32 = port_check::free_local_port().unwrap().into();
            create_options.expose(expose_port.srcport, "tcp", port);

            let value = (*expose_port.env_file_value)(port);

            if expose_port.for_client {
                http_url = Some(value.clone());
            }

            to_write_in_env.push((expose_port.env_file_key, value));
        }
        let http_url: String = http_url.unwrap_or_else(|| {
            panic!("Internal Error: Url for client should have been set.");
        });
        let create_options = create_options.build();
        (to_write_in_env, http_url, create_options)
    }

    fn start_container(
        envfile_path: PathBuf,
        create_options: ContainerOptions,
        client_endpoint: String,
        to_write_in_env: Vec<(String, String)>,
    ) -> impl Future<Item = Self, Error = shiplift::errors::Error> {
        Docker::new()
            .containers()
            .create(&create_options)
            .map_err(|e| {
                eprintln!("Error encountered when creating container: {:?}", e);
                e
            })
            .and_then({
                move |container| {
                    let id = container.id;
                    Docker::new()
                        .containers()
                        .get(&id)
                        .start()
                        .map(|_| id)
                        .map_err(|e| {
                            eprintln!("Error encountered when starting container: {:?}", e);
                            e
                        })
                }
            })
            .and_then({
                move |id| {
                    Docker::new()
                        .containers()
                        .get(&id)
                        .logs(
                            &LogsOptions::builder()
                                .stdout(true)
                                .stderr(true)
                                .follow(true)
                                .build(),
                        )
                        .take_while(|chunk| {
                            let log = chunk.as_string_lossy();
                            Ok(!log.contains(I::LOG_READY))
                        })
                        .collect()
                        .map_err(|e| {
                            eprintln!("Error encountered when getting logs: {:?}", e);
                            e
                        })
                        .map(|_| id)
                }
            })
            .and_then({
                let http_url = client_endpoint.clone();
                move |container_id| {
                    Ok(Self {
                        container_id,
                        node_image: I::new(http_url),
                    })
                }
            })
            .and_then({
                let envfile_path = envfile_path.clone();
                move |node| {
                    let mut envfile = EnvFile::new(envfile_path).unwrap();
                    for key_value in to_write_in_env {
                        envfile.update(&key_value.0, &key_value.1).write().unwrap();
                    }

                    Ok(node)
                }
            })
    }

    pub fn stop_remove(&self) -> impl Future<Item = (), Error = ()> {
        Docker::new()
            .containers()
            .get(&self.container_id)
            .remove(
                RmContainerOptions::builder()
                    .force(true)
                    .volumes(true)
                    .build(),
            )
            .map_err(|_| ())
    }
}

impl<I: Image> Drop for Node<I> {
    fn drop(&mut self) {
        let rm_fut = Docker::new()
            .containers()
            .get(&self.container_id)
            .remove(
                RmContainerOptions::builder()
                    .force(true)
                    .volumes(true)
                    .build(),
            )
            .map_err(|_| ());

        tokio::run(rm_fut);
    }
}

pub fn create_network() -> impl Future<Item = String, Error = shiplift::Error> {
    Docker::new()
        .networks()
        .get(DOCKER_NETWORK)
        .inspect()
        .map(|info| {
            eprintln!(
                "\n[warn] {} Docker network already exist, re-using it.",
                DOCKER_NETWORK
            );
            info.id
        })
        .or_else(|_| {
            Docker::new()
                .networks()
                .create(
                    &NetworkCreateOptions::builder(DOCKER_NETWORK)
                        .driver("bridge")
                        .build(),
                )
                .and_then(|info| Ok(info.id))
        })
}

pub fn delete_network(id: String) -> impl Future<Item = (), Error = shiplift::Error> {
    Docker::new().networks().get(id.clone().as_str()).delete()
}
