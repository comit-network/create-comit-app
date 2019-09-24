use envfile::EnvFile;
use futures::stream::Stream;
use futures::Future;
use shiplift::{ContainerOptions, Docker, LogsOptions, PullOptions, RmContainerOptions};
use std::path::PathBuf;

pub mod bitcoin;
pub mod ethereum;

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

pub struct Node<I: BlockchainImage> {
    container_id: String,
    pub node_image: I,
}

// TODO: Move all envfile stuff outside
// TODO: Move free_local_port outside
impl<I: BlockchainImage> Node<I> {
    pub fn start(
        envfile_path: PathBuf,
    ) -> impl Future<Item = Self, Error = shiplift::errors::Error> {
        let docker = Docker::new();
        docker
            .images()
            .pull(&PullOptions::builder().image(I::IMAGE).build())
            // TODO: Pretty print progress
            .collect()
            .and_then(|_| Self::start_container(envfile_path))
            .inspect(|node| {
                node.node_image.post_start_actions();
            })
    }

    fn start_container(
        envfile_path: PathBuf,
    ) -> impl Future<Item = Self, Error = shiplift::errors::Error> {
        let docker = Docker::new();

        let mut create_options = ContainerOptions::builder(I::IMAGE);
        create_options.cmd(I::arguments_for_create());

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
        docker
            .containers()
            .create(&create_options)
            .and_then({
                let docker = docker.clone();
                move |container| {
                    let id = container.id;
                    docker.containers().get(&id).start().map(|_| id)
                }
            })
            .and_then({
                let docker = docker.clone();
                move |id| {
                    docker
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
                        .map(|_| id)
                }
            })
            .and_then({
                let http_url = http_url.clone();
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

impl<I: BlockchainImage> Drop for Node<I> {
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
