use crate::{filesystem, network, Result};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Represents a version with channel, name and path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub channel: Channel,
    pub base_server_url: String,

    pub directory: PathBuf,
    pub version: String,
    pub newer_version: Option<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Profile::new(
            "default".to_owned(),
            Channel::Nightly,
            network::DOWNLOAD_SERVER.to_owned(),
        )
    }
}

#[derive(Debug, Display, Clone, Copy, Serialize, Deserialize)]
pub enum Channel {
    Nightly,
    // TODO: Release,
    // TODO: Source,
}

impl Profile {
    /// Creates a new profile and downloads the correct files into the target directory.
    pub fn new(name: String, channel: Channel, base_server_url: String) -> Self {
        Self {
            directory: filesystem::get_profile_path(&name),
            name,
            channel,
            base_server_url,
            version: "".to_owned(), // Will be set by download
            newer_version: None,
        }
    }

    pub fn start_download(&self) -> Result<(isahc::Metrics, PathBuf)> {
        network::start_download(&self)
    }

    pub async fn install(mut self, zip_path: PathBuf) -> Result<Profile> {
        if let Some(newer_version) = &self.newer_version {
            network::install(&self, zip_path).await?;
            self.version = newer_version.clone();
            self.newer_version = None;
            Ok(self)
        } else {
            Err("No newer version found".into())
        }
    }

    // TODO: add possibility to start the server too
    pub async fn start(&self) -> Result<()> {
        let mut envs = HashMap::new();
        envs.insert("VOXYGEN_CONFIG", self.directory.clone().into_os_string());

        log::debug!("Launching {}", self.voxygen_path().display());
        log::debug!("CWD: {:?}", self.directory);
        log::debug!("ENV: {:?}", envs);

        let cmd = Command::new(self.voxygen_path())
            .current_dir(&self.directory)
            .envs(envs)
            .status()?;
        log::debug!(
            "Veloren exited with code: {}",
            cmd.code()
                .map(|x| x.to_string())
                .unwrap_or("Exit code unavailable.".to_string())
        );
        Ok(())
    }

    pub async fn check_for_update(&mut self) -> Result<()> {
        let remote_version = network::get_newest_version_name(&self).await?;
        if self.version != remote_version {
            self.newer_version = Some(remote_version);
        } else {
            self.newer_version = None;
        }
        Ok(())
    }

    /// Returns path to voxygen binary.
    /// e.g. <base>/profiles/latest/veloren-voxygen.exe
    fn voxygen_path(&self) -> PathBuf {
        self.directory.join(filesystem::VOXYGEN_FILE)
    }
}
