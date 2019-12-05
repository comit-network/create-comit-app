use crate::print_progress;
use futures::{
    compat::{Future01CompatExt, Stream01CompatExt},
    StreamExt, TryStreamExt,
};
use shiplift::{
    ContainerOptions, Docker, LogsOptions, NetworkCreateOptions, PullOptions, RmContainerOptions,
};

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
        let _ = docker
            .images()
            .pull(&options)
            .compat()
            .try_collect::<Vec<_>>();
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
        .compat()
        .take_while(|chunk| {
            let log = match chunk {
                Ok(chunk) => chunk.as_string_lossy(),
                Err(_) => return futures::future::ready(false),
            };

            futures::future::ready(!log.contains(wait_for.0))
        })
        .try_collect::<Vec<_>>()
        .await;

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

pub async fn delete_network() -> anyhow::Result<()> {
    Docker::new()
        .networks()
        .get(DOCKER_NETWORK)
        .delete()
        .compat()
        .await?;

    Ok(())
}

pub async fn delete_container(name: &str) -> anyhow::Result<()> {
    Docker::new()
        .containers()
        .get(name)
        .remove(
            RmContainerOptions::builder()
                .force(true)
                .volumes(true)
                .build(),
        )
        .compat()
        .await?;

    Ok(())
}
