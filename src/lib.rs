use anyhow::Result;
use serde::Serialize;

pub mod adapters;
pub mod application;
pub mod domain;
pub mod error;
pub mod interfaces;
pub mod ports;
pub mod support;

use support::{
    cli::{AppCommand, DoctorMode},
    config::{AppConfig, TransportKind},
    doctor::DoctorReport,
};

pub fn startup_transport_from_default_config() -> TransportKind {
    AppConfig::default().transport
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum RunOutput {
    Doctor(Box<DoctorReport>),
    DatabaseLifecycle(Box<adapters::sqlite::DatabaseLifecycleReport>),
}

pub async fn run_command(command: AppCommand, config: AppConfig) -> Result<Option<RunOutput>> {
    match command {
        AppCommand::Serve => {
            match config.transport {
                TransportKind::Stdio => {
                    interfaces::mcp::run_stdio_server_with_config(config).await?
                }
            }

            Ok(None)
        }
        AppCommand::Init => adapters::sqlite::initialize_database(&config.database_url)
            .await
            .map(|report| Some(RunOutput::DatabaseLifecycle(Box::new(report))))
            .map_err(anyhow::Error::from),
        AppCommand::Migrate => adapters::sqlite::migrate_database(&config.database_url)
            .await
            .map(|report| Some(RunOutput::DatabaseLifecycle(Box::new(report))))
            .map_err(anyhow::Error::from),
        AppCommand::Doctor(DoctorMode::ReadOnly) => run_doctor(config)
            .await
            .map(|report| Some(RunOutput::Doctor(Box::new(report)))),
        AppCommand::Doctor(DoctorMode::AllowBootstrap) => {
            support::doctor::run_doctor_allow_bootstrap(config)
                .await
                .map(|report| Some(RunOutput::Doctor(Box::new(report))))
        }
    }
}

pub async fn run_doctor(config: AppConfig) -> Result<DoctorReport> {
    support::doctor::run_doctor(config).await
}

pub async fn run() -> Result<()> {
    support::tracing::init_tracing();
    let config = AppConfig::load().map_err(anyhow::Error::msg)?;
    run_command(AppCommand::Serve, config).await.map(|_| ())
}
