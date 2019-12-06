use crate::print_progress;
use futures::compat::Future01CompatExt;
use shiplift::{
    ContainerOptions, Docker, LogsOptions, NetworkCreateOptions, PullOptions, RmContainerOptions,
};
use tokio::prelude::{stream::Stream, Future};

pub mod bitcoin;
pub mod cnd;
pub mod ethereum;
mod free_local_port;

pub const DOCKER_NETWORK: &str = "create-comit-app";

pub struct DockerImage(pub &'static str);
pub struct LogMessage(pub &'static str);

pub async fn start(
    image: DockerImage,
    options: ContainerOptions,
    wait_for: LogMessage,
) -> anyhow::Result<()> {
    let docker = Docker::new();

    let images = docker.images().list(&Default::default()).compat().await?;

    let image_is_present_locally = images
        .iter()
        .find(|local_image| {
            local_image
                .repo_tags
                .as_ref()
                .and_then(|repo_tags| repo_tags.iter().find(|tag| *tag == image.0).map(|_| true))
                .unwrap_or(false)
        })
        .map(|_| true)
        .unwrap_or(false);

    if !image_is_present_locally {
        print_progress!("Downloading {}", image.0);
        let options = PullOptions::builder().image(image.0).build();
        docker.images().pull(&options).collect().compat().await?;
    }

    let container = docker.containers().create(&options).compat().await?;

    let container = docker.containers().get(&container.id);

    container.start().compat().await?;

    let _ = container
        .logs(
            &LogsOptions::builder()
                .stdout(true)
                .stderr(true)
                .follow(true)
                .build(),
        )
        .take_while(|chunk| {
            let log = chunk.as_string_lossy();
            Ok(!log.contains(wait_for.0))
        })
        .collect()
        .compat()
        .await?;

    Ok(())
}

pub async fn create_network() -> anyhow::Result<String> {
    let docker = Docker::new();

    let response = docker
        .networks()
        .get(DOCKER_NETWORK)
        .inspect()
        .compat()
        .await;

    if let Ok(info) = response {
        eprintln!(
            "\n[warn] {} Docker network already exist, re-using it.",
            DOCKER_NETWORK
        );

        return Ok(info.id);
    }

    let response = docker
        .networks()
        .create(
            &NetworkCreateOptions::builder(DOCKER_NETWORK)
                .driver("bridge")
                .build(),
        )
        .compat()
        .await?;

    Ok(response.id)
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
