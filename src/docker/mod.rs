use crate::print_progress;
use anyhow::Context;
use futures::compat::Future01CompatExt;
use futures_01::stream::Stream;
use shiplift::{
    ContainerOptions, Docker, LogsOptions, NetworkCreateOptions, PullOptions, RmContainerOptions,
};
use std::{net::Ipv4Addr, path::Path};

pub mod bitcoin;
pub mod cnd;
pub mod ethereum;
mod free_local_port;

pub const DOCKER_NETWORK: &str = "create-comit-app";

pub struct DockerImage(pub &'static str);
pub struct LogMessage(pub &'static str);

/// A file that should be copied into the container before it is started
pub struct File<'a> {
    location: &'a Path,
    content: &'a [u8],
}

pub async fn start(
    image: DockerImage,
    options: ContainerOptions,
    wait_for: LogMessage,
    files: Vec<File<'_>>,
) -> anyhow::Result<()> {
    let docker = Docker::new();

    let images = docker
        .images()
        .list(&Default::default())
        .compat()
        .await
        .context("unable to list local docker images")?;

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
        docker
            .images()
            .pull(&options)
            .collect()
            .compat()
            .await
            .context("failed to pull image")?;
    }

    let container = docker
        .containers()
        .create(&options)
        .compat()
        .await
        .context("failed to create container")?;

    let container = docker.containers().get(&container.id);

    for file in files {
        container
            .copy_file_into(file.location, file.content)
            .compat()
            .await?;
    }

    container
        .start()
        .compat()
        .await
        .context("failed to start container")?;

    container
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
        .await
        .context("failed while waiting for container to be ready")?;

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
        .await
        .with_context(|| format!("failed to created docker network {}", DOCKER_NETWORK))?;

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

pub fn docker_daemon_ip() -> anyhow::Result<Ipv4Addr> {
    let socket = match std::env::var("DOCKER_HOST") {
        Ok(host) => parse_ip(host)?,
        Err(_) => Ipv4Addr::LOCALHOST,
    };

    Ok(socket)
}

fn parse_ip(uri: String) -> anyhow::Result<Ipv4Addr> {
    let uri = uri.parse::<http::Uri>()?;
    let host = uri
        .host()
        .ok_or_else(|| anyhow::anyhow!("DOCKER_HOST {} is not a URI with a host"))?;
    let ip = host
        .parse()
        .with_context(|| format!("{} is not a valid ipv4 address", host))?;

    Ok(ip)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn can_parse_ip_from_docker_host() {
        let docker_host = "tcp://192.168.99.100:2376";

        let ip = parse_ip(docker_host.to_string()).unwrap();

        assert_eq!(ip, Ipv4Addr::new(192, 168, 99, 100));
    }
}
