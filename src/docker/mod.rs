use crate::print_progress;
use envfile::EnvFile;
use shiplift::builder::ContainerOptionsBuilder;
use shiplift::{
    ContainerOptions, Docker, LogsOptions, NetworkCreateOptions, PullOptions, RmContainerOptions,
};
use std::path::PathBuf;
use tokio::prelude::future::Either;
use tokio::prelude::stream::Stream;
use tokio::prelude::Future;

pub mod blockchain;
pub mod cnd;

pub use self::blockchain::{bitcoin, ethereum};
pub use self::cnd::Cnd;

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
    fn expose_ports(name: &str) -> Vec<ExposedPorts>;
    fn new(endpoint: Option<String>) -> Self;
}

pub struct Node<I: Image> {
    pub node_image: I,
}

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
            <Node<I>>::write_env_file(&name, &mut create_options);

        Self::get_image().and_then(move |_| {
            Self::start_container(
                envfile_path,
                create_options,
                client_endpoint,
                to_write_in_env,
            )
        })
    }

    pub fn start_with_volume(
        envfile_path: PathBuf,
        name: &str,
        volume: &str,
    ) -> impl Future<Item = Self, Error = shiplift::Error> {
        let mut create_options = ContainerOptions::builder(I::IMAGE);
        create_options.name(&name);
        create_options.network_mode(DOCKER_NETWORK);
        create_options.cmd(I::arguments_for_create());
        create_options.volumes(vec![volume]);

        let (to_write_in_env, client_endpoint, create_options) =
            <Node<I>>::write_env_file(&name, &mut create_options);

        Self::get_image().and_then(move |_| {
            Self::start_container(
                envfile_path,
                create_options,
                client_endpoint,
                to_write_in_env,
            )
        })
    }

    fn get_image() -> impl Future<Item = (), Error = shiplift::Error> {
        Docker::new()
            .images()
            .list(&Default::default())
            .map(|images| {
                images
                    .iter()
                    .find(|image| {
                        image
                            .repo_tags
                            .as_ref()
                            .and_then(|repo_tags| {
                                repo_tags
                                    .iter()
                                    .find(|tag| *tag == &I::IMAGE.to_string())
                                    .map(|_| true)
                            })
                            .unwrap_or(false)
                    })
                    .map(|_| true)
                    .unwrap_or(false)
            })
            .and_then(|image_present| {
                if !image_present {
                    print_progress!("Downloading {}", I::IMAGE);
                    Either::A(
                        Docker::new()
                            .images()
                            .pull(&PullOptions::builder().image(I::IMAGE).build())
                            .collect()
                            .map(|_| ()),
                    )
                } else {
                    Either::B(tokio::prelude::future::ok(()))
                }
            })
    }

    fn write_env_file(
        name: &str,
        create_options: &mut ContainerOptionsBuilder,
    ) -> (Vec<(String, String)>, Option<String>, ContainerOptions) {
        let mut to_write_in_env: Vec<(String, String)> = vec![];
        let mut http_url: Option<String> = None;
        for expose_port in I::expose_ports(name) {
            let port: u32 = port_check::free_local_port().unwrap().into();
            create_options.expose(expose_port.srcport, "tcp", port);

            let value = (*expose_port.env_file_value)(port);

            if expose_port.for_client {
                http_url = Some(value.clone());
            }

            to_write_in_env.push((expose_port.env_file_key, value));
        }
        let create_options = create_options.build();
        (to_write_in_env, http_url, create_options)
    }

    fn start_container(
        envfile_path: PathBuf,
        create_options: ContainerOptions,
        client_endpoint: Option<String>,
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
                move |_| {
                    Ok(Self {
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

pub fn delete_network() -> impl Future<Item = (), Error = shiplift::Error> {
    Docker::new().networks().get(DOCKER_NETWORK).delete()
}

pub fn delete_container(name: &str) -> impl Future<Item = (), Error = shiplift::Error> {
    Docker::new().containers().get(name).remove(
        RmContainerOptions::builder()
            .force(true)
            .volumes(true)
            .build(),
    )
}
