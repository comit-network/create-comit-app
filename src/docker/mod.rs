use envfile::EnvFile;
use futures::stream::Stream;
use futures::Future;
use shiplift::{ContainerOptions, Docker, LogsOptions, PullOptions, RmContainerOptions};
use std::path::PathBuf;

pub mod bitcoin;
pub mod ethereum;

pub trait NodeImage {
    const IMAGE: &'static str;
    const HTTP_URL_KEY: &'static str;
    type Address;
    type Amount;
    type TxId;
    type ClientError;

    fn arguments_for_create() -> Vec<&'static str>;
    fn client_port() -> u32;
    fn new(endpoint: String) -> Self;
    fn fund(
        &self,
        address: Self::Address,
        value: Self::Amount,
    ) -> Box<dyn Future<Item = Self::TxId, Error = Self::ClientError> + Send + Sync>;
    fn log_ready() -> String;
}

pub struct Node<I: NodeImage> {
    container_id: String,
    pub node_image: I,
}

// TODO: Move all envfile stuff outside
// TODO: Move free_local_port outside
impl<I: NodeImage> Node<I> {
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
    }

    fn start_container(
        envfile_path: PathBuf,
    ) -> impl Future<Item = Self, Error = shiplift::errors::Error> {
        let http_port: u32 = port_check::free_local_port().unwrap().into();
        let http_url = format!("http://localhost:{}", http_port);

        let docker = Docker::new();
        let create_options = ContainerOptions::builder(I::IMAGE)
            .cmd(I::arguments_for_create())
            .expose(I::client_port(), "tcp", http_port)
            .build();
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
                            Ok(!log.contains(I::log_ready().as_str()))
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
                let http_url = http_url.clone();
                move |node| {
                    let mut envfile = EnvFile::new(envfile_path).unwrap();
                    envfile.update(&I::HTTP_URL_KEY, &http_url).write().unwrap();

                    Ok(node)
                }
            })
    }
}

impl<I: NodeImage> Drop for Node<I> {
    fn drop(&mut self) {
        let docker = Docker::new();

        let rm_fut = docker
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
