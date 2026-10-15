use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    Serve,
    Init,
    Migrate,
    Doctor(DoctorMode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorMode {
    ReadOnly,
    AllowBootstrap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliError {
    message: String,
}

impl CliError {
    fn unsupported_command(command: &str) -> Self {
        Self {
            message: format!("unsupported command: {command}"),
        }
    }

    fn invalid_arguments(command: &str) -> Self {
        Self {
            message: format!("invalid arguments for {command}"),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for CliError {}

pub fn command_from_args<I>(args: I) -> Result<AppCommand, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let _program_name = args.next();

    match args.next().as_deref() {
        None => Ok(AppCommand::Serve),
        Some("serve") if args.next().is_none() => Ok(AppCommand::Serve),
        Some("init") if args.next().is_none() => Ok(AppCommand::Init),
        Some("migrate") if args.next().is_none() => Ok(AppCommand::Migrate),
        Some("doctor") => match (args.next().as_deref(), args.next()) {
            (None | Some("--read-only"), None) => Ok(AppCommand::Doctor(DoctorMode::ReadOnly)),
            (Some("--allow-bootstrap"), None) => Ok(AppCommand::Doctor(DoctorMode::AllowBootstrap)),
            _ => Err(CliError::invalid_arguments("doctor")),
        },
        Some("serve" | "init" | "migrate") => Err(CliError::invalid_arguments("command")),
        Some(other) => Err(CliError::unsupported_command(other)),
    }
}
