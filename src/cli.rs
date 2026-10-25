// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{Config, DestinationConfig},
    diagnostics::Diagnostics,
    domain::{self, Detector, PolicySet, ShellExec},
    error::{Error, Result, join},
    io::{OutputFile, ReplayFile, Writer},
    output::{Destination, Json, Table, Webhook},
    pipeline::{self, Options},
    replay::{Collector, Replay},
};
use clap::{Arg, ArgAction, Command};
use std::{
    fs::{self, OpenOptions},
    sync::Arc,
};
use tokio_util::sync::CancellationToken;

fn sensor_command() -> Command {
    Command::new("buoybsd")
        .args_override_self(true)
        .disable_version_flag(true)
        .disable_help_flag(true)
        .arg(
            Arg::new("config")
                .long("config")
                .default_value("")
                .allow_hyphen_values(true)
                .help("YAML configuration file"),
        )
        .arg(
            Arg::new("replay")
                .long("replay")
                .default_value("")
                .allow_hyphen_values(true)
                .help("synthetic replay NDJSON file; no live collector is implemented"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .default_value("json")
                .allow_hyphen_values(true)
                .help("stdout format: json or table"),
        )
        .arg(
            Arg::new("version")
                .long("version")
                .action(ArgAction::Set)
                .num_args(0..=1)
                .require_equals(true)
                .default_value("false")
                .default_missing_value("true")
                .value_parser(go_bool)
                .help("print version without collecting"),
        )
        .arg(
            Arg::new("help")
                .long("help")
                .short('h')
                .action(ArgAction::SetTrue),
        )
}

fn go_bool(value: &str) -> std::result::Result<bool, &'static str> {
    match value {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Ok(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Ok(false),
        _ => Err("invalid boolean value"),
    }
}

fn legacy_flags(args: &[String]) -> Vec<String> {
    let mut value = false;
    let mut positional = false;
    args.iter()
        .map(|arg| {
            if value {
                value = false;
                return arg.clone();
            }
            if positional {
                return arg.clone();
            }
            if arg == "--" {
                positional = true;
                return arg.clone();
            }
            let name = arg.split('=').next().unwrap_or("");
            value = !arg.contains('=')
                && [
                    "-config", "--config", "-replay", "--replay", "-output", "--output",
                ]
                .contains(&name);
            if ["-config", "-replay", "-output", "-version", "-help"].contains(&name) {
                format!("-{arg}")
            } else {
                arg.clone()
            }
        })
        .collect()
}

pub async fn run_sensor(
    cancel: &CancellationToken,
    args: &[String],
    mut stdout: Box<dyn Writer>,
    diagnostics: &Diagnostics,
) -> Result<()> {
    let mut command = sensor_command();
    let flags = match command
        .clone()
        .try_get_matches_from(std::iter::once("buoybsd".to_string()).chain(legacy_flags(args)))
    {
        Ok(flags) => flags,
        Err(error) => {
            diagnostics.plain(&error.to_string());
            return Err(
                if error.kind() == clap::error::ErrorKind::UnknownArgument
                    && args.iter().any(|s| !s.starts_with('-'))
                {
                    Error::message("buoybsd does not accept positional arguments")
                } else {
                    Error::message(error.to_string())
                },
            );
        }
    };
    if flags.get_flag("help") {
        diagnostics.plain(&format!("{}\n", command.render_help()));
        return Ok(());
    }
    if *flags
        .get_one::<bool>("version")
        .expect("version has a default")
    {
        writeln!(
            stdout,
            "buoybsd {} ({}/{}; replay prototype)",
            domain::VERSION,
            platform_os(),
            platform_arch()
        )
        .map_err(|e| Error::io("write version", e))?;
        return Ok(());
    }
    let config_path = flags.get_one::<String>("config").expect("config default");
    let mut config = if config_path.is_empty() {
        Config::default()
    } else {
        load_config(config_path)?
    };
    if flags.value_source("replay") == Some(clap::parser::ValueSource::CommandLine) {
        config.collector.path = flags
            .get_one::<String>("replay")
            .expect("replay default")
            .clone();
    }
    if flags.value_source("output") == Some(clap::parser::ValueSource::CommandLine) {
        let format = flags.get_one::<String>("output").expect("output default");
        if format != "json" && format != "table" {
            return Err(Error::message("output must be json or table"));
        }
        let mut replaced = false;
        for output in &mut config.outputs {
            if output.kind == "stdout" {
                output.format.clone_from(format);
                replaced = true;
            }
        }
        if !replaced {
            config.outputs.push(DestinationConfig {
                name: "cli-stdout".into(),
                kind: "stdout".into(),
                format: format.clone(),
                ..DestinationConfig::default()
            });
        }
    }
    config
        .validate()
        .map_err(|e| e.context("validate configuration"))?;
    config
        .validate_paths(config_path)
        .map_err(|e| e.context("validate paths"))?;
    let policies = PolicySet::compile(config.policies)?;
    let file =
        fs::File::open(&config.collector.path).map_err(|e| Error::io("open replay input", e))?;
    let source = Arc::new(Replay::new(
        Box::new(ReplayFile::new(file)),
        config.collector.max_record_bytes,
    )?);
    let destinations = match open_destinations(config.outputs, stdout).await {
        Ok(destinations) => destinations,
        Err(error) => {
            let mut result = Err(error);
            join(&mut result, source.close().await);
            return result;
        }
    };
    let detectors: Vec<Box<dyn Detector>> = if config.detectors.is_empty() {
        vec![]
    } else {
        vec![Box::new(ShellExec)]
    };
    let (stats, result) = pipeline::run(
        cancel,
        source,
        destinations,
        Options {
            queue_size: config.pipeline.queue_size,
            policies,
            detectors,
        },
    )
    .await;
    diagnostics.stopped("replay", stats, result.is_err());
    result
}

fn load_config(path: &str) -> Result<Config> {
    let info = fs::metadata(path).map_err(|e| Error::io("stat configuration", e))?;
    if !info.is_file() {
        return Err(Error::message("configuration must be a regular file"));
    }
    let file = fs::File::open(path).map_err(|e| Error::io("open configuration", e))?;
    let result = Config::load(&file);
    let closed = crate::io::close_file(file).map_err(|e| Error::io("close configuration", e));
    match (result, closed) {
        (Ok(config), Ok(())) => Ok(config),
        (Err(error), closed) => {
            let mut result = Err(error);
            join(&mut result, closed);
            result.map(|()| unreachable!())
        }
        (Ok(_), Err(error)) => Err(error),
    }
}

async fn open_destinations(
    specs: Vec<DestinationConfig>,
    stdout: Box<dyn Writer>,
) -> Result<Vec<Box<dyn Destination>>> {
    let mut stdout = Some(stdout);
    let mut destinations: Vec<Box<dyn Destination>> = vec![];
    for spec in specs {
        let name = spec.name.clone();
        let destination = (|| -> Result<Box<dyn Destination>> {
            match spec.kind.as_str() {
                "stdout" => {
                    let writer = stdout
                        .take()
                        .ok_or_else(|| Error::message("only one stdout output is allowed"))?;
                    if spec.format == "table" {
                        Ok(Box::new(Table::new(spec.name, writer)?))
                    } else {
                        Ok(Box::new(Json::new(spec.name, writer, false)?))
                    }
                }
                "file" => {
                    let mut options = OpenOptions::new();
                    options.create(true).append(true);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::OpenOptionsExt;
                        options.mode(0o600);
                    }
                    let file = options
                        .open(spec.path)
                        .map_err(|e| Error::io("open file output", e))?;
                    Ok(Box::new(Json::new(
                        spec.name,
                        Box::new(OutputFile::new(file)),
                        true,
                    )?))
                }
                "webhook" => Ok(Box::new(Webhook::new(spec)?)),
                _ => Err(Error::message("unsupported destination type")),
            }
        })();
        match destination {
            Ok(destination) => destinations.push(destination),
            Err(error) => {
                let mut result = Err(error.context(format!("open destination {name:?}")));
                for destination in &mut destinations {
                    join(&mut result, destination.close().await);
                }
                return result.map(|()| destinations);
            }
        }
    }
    Ok(destinations)
}

pub fn run_control(args: &[String], stdout: &mut dyn Writer) -> Result<()> {
    let usage = "usage: buoyctl version | events list | status (not implemented)";
    let args: Vec<String> = if args == ["--version"] {
        vec!["version".into()]
    } else {
        args.to_vec()
    };
    let command = Command::new("buoyctl")
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .subcommand(Command::new("version"))
        .subcommand(Command::new("events").subcommand(Command::new("list")))
        .subcommand(Command::new("status"));
    let flags = command
        .try_get_matches_from(std::iter::once("buoyctl".into()).chain(args))
        .map_err(|_| Error::message(usage))?;
    match flags.subcommand() {
        Some(("version", _)) => writeln!(stdout, "buoyctl {}", domain::VERSION)
            .map_err(|e| Error::io("write version", e)),
        Some(("events", events)) if events.subcommand_name() == Some("list") => {
            let mut data = serde_json::to_vec(&domain::definitions())
                .map_err(|_| Error::message("serialize event catalogue"))?;
            data.push(b'\n');
            stdout
                .write_all(&data)
                .map_err(|e| Error::io("write event catalogue", e))
        }
        Some(("status", _)) => Err(Error::message(
            "status unavailable: no live collector or control service is implemented",
        )),
        _ => Err(Error::message(usage)),
    }
}

fn platform_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        os => os,
    }
}
fn platform_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "x86" => "386",
        "aarch64" => "arm64",
        arch => arch,
    }
}
