// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Parser)]
struct Args {
    #[arg(value_enum)]
    command: Operation,
}

#[derive(Clone, ValueEnum)]
enum Operation {
    Generate,
    CheckGenerated,
    Notices,
    License,
    Clean,
}

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::env::set_current_dir(&root)?;
    let scratch = root.join(".tools").join("protobuf-check");
    match Args::parse().command {
        Operation::Notices => buoybsd::dev::notices(true),
        Operation::License => buoybsd::dev::license(),
        Operation::Clean => buoybsd::dev::clean(),
        Operation::Generate => generate(&root.join("api").join("v1beta1")),
        Operation::CheckGenerated => {
            if scratch.exists() {
                fs::remove_dir_all(&scratch)?;
            }
            generate(&scratch)?;
            let comparison = (|| {
                for name in ["buoybsd.v1beta1.rs", "buoybsd.v1beta1.serde.rs"] {
                    if fs::read(scratch.join(name))?
                        != fs::read(root.join("api").join("v1beta1").join(name))?
                    {
                        bail!("generated protobuf differs; run make generate");
                    }
                }
                println!("Generated Rust protobuf and JSON bindings are reproducible.");
                Ok(())
            })();
            fs::remove_dir_all(&scratch)?;
            comparison
        }
    }
}

fn generate(output: &Path) -> Result<()> {
    let protoc = std::env::var_os("PROTOC").unwrap_or_else(|| "protoc".into());
    let version = Command::new(&protoc)
        .arg("--version")
        .output()
        .context("protoc 36.2 is required; install the checksum-verified official archive")?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != "libprotoc 36.2"
    {
        bail!("protobuf generation requires exactly protoc 36.2");
    }
    fs::create_dir_all(output)?;
    let descriptor = output.join("descriptor.bin");
    let mut config = prost_build::Config::new();
    config
        .protoc_executable(&protoc)
        .out_dir(output)
        .file_descriptor_set_path(&descriptor)
        .compile_well_known_types()
        .extern_path(".google.protobuf.Timestamp", "crate::wire::Timestamp");
    let mut includes = vec![PathBuf::from(".")];
    if let Some(include) = std::env::var_os("PROTOC_INCLUDE") {
        includes.push(include.into());
    } else if let Some(parent) = Path::new(&protoc).parent().and_then(Path::parent) {
        includes.push(parent.join("include"));
    }
    config.compile_protos(&["api/v1beta1/event.proto"], &includes)?;
    pbjson_build::Builder::new()
        .out_dir(output)
        .register_descriptors(&fs::read(&descriptor)?)?
        .extern_path(".google.protobuf.Timestamp", "crate::wire::Timestamp")
        .preserve_proto_field_names()
        .build(&[".buoybsd.v1beta1"])?;
    let json_path = output.join("buoybsd.v1beta1.serde.rs");
    let json = fs::read_to_string(&json_path)?;
    // pbjson 0.9 emits an unnecessary borrow rejected by Rust 1.99's strict Clippy.
    let json = json.replace(
        "write!(formatter, \"expected one of: {:?}\", &FIELDS)",
        "write!(formatter, \"expected one of: {:?}\", FIELDS)",
    );
    fs::write(json_path, json)?;
    fs::remove_file(descriptor)?;
    Ok(())
}
