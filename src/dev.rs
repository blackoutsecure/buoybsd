// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Resolve,
}
#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    license: Option<String>,
    license_file: Option<String>,
    manifest_path: PathBuf,
    source: Option<String>,
}
#[derive(Deserialize)]
struct Resolve {
    root: String,
    nodes: Vec<Node>,
}
#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<Dependency>,
}
#[derive(Deserialize)]
struct Dependency {
    pkg: String,
    dep_kinds: Vec<Kind>,
}
#[derive(Deserialize)]
struct Kind {
    kind: Option<String>,
}

#[derive(Serialize)]
struct DependencyNotice {
    name: String,
    version: String,
    license: String,
    source: Option<String>,
    license_source: Option<String>,
    files: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct UpstreamNotice {
    name: String,
    version: String,
    commit: String,
    source_url: String,
    sha256: String,
    text: Vec<String>,
}
#[derive(Serialize)]
struct NoticeIndex {
    rustc: String,
    rust_library_files: BTreeMap<String, String>,
    dependencies: Vec<DependencyNotice>,
}

pub fn capture(program: &std::ffi::OsStr, args: &[&str]) -> Result<Vec<u8>> {
    eprintln!("> {} {}", program.to_string_lossy(), args.join(" "));
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("start developer tool")?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let read = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(32 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        Ok::<_, std::io::Error>(bytes)
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() >= Duration::from_secs(600) {
            child.kill()?;
            let _ = child.wait();
            bail!("developer metadata command exceeded its 600-second deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let bytes = read
        .join()
        .map_err(|_| anyhow::anyhow!("tool stdout reader failed"))??;
    if !status.success() {
        bail!("developer tool failed: {status}");
    }
    if bytes.len() > 32 * 1024 * 1024 {
        bail!("tool metadata exceeded 32 MiB");
    }
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn portable(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn license_files(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if [
            "license",
            "licence",
            "copying",
            "copyright",
            "notice",
            "authors",
        ]
        .iter()
        .any(|prefix| name.starts_with(prefix))
        {
            if entry.file_type()?.is_file() {
                files.push(entry.path());
            } else if entry.file_type()?.is_dir() {
                collect_files(&entry.path(), &mut files)?;
            }
        }
    }
    Ok(files)
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            files.push(entry.path());
        } else if entry.file_type()?.is_dir() {
            collect_files(&entry.path(), files)?;
        } else {
            bail!("refusing a symlink in generated notice inputs");
        }
    }
    Ok(())
}

fn expected_notices() -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let metadata: Metadata = serde_json::from_slice(&capture(
        &cargo,
        &[
            "metadata",
            "--locked",
            "--offline",
            "--no-default-features",
            "--format-version",
            "1",
        ],
    )?)?;
    let mut included = HashSet::new();
    let mut pending = vec![metadata.resolve.root.clone()];
    let nodes: BTreeMap<_, _> = metadata.resolve.nodes.iter().map(|n| (&n.id, n)).collect();
    while let Some(id) = pending.pop() {
        if !included.insert(id.clone()) {
            continue;
        }
        if let Some(node) = nodes.get(&id) {
            for dep in &node.deps {
                if dep.dep_kinds.iter().any(|k| k.kind.is_none()) {
                    pending.push(dep.pkg.clone());
                }
            }
        }
    }
    let mut snapshots = BTreeMap::new();
    let mut dependencies = vec![];
    let mut missing = vec![];
    let upstream: Vec<UpstreamNotice> =
        serde_json::from_slice(&fs::read("licenses/upstream-sources.json")?)?;
    for package in metadata
        .packages
        .iter()
        .filter(|p| p.id != metadata.resolve.root && included.contains(&p.id))
    {
        let directory = package
            .manifest_path
            .parent()
            .context("dependency manifest has no directory")?;
        let directory = fs::canonicalize(directory)?;
        let mut files = license_files(&directory)?;
        if let Some(file) = &package.license_file {
            let file = PathBuf::from(file);
            files.push(if file.is_absolute() {
                file
            } else {
                directory.join(file)
            });
        }
        files.sort();
        files.dedup();
        let mut hashes = BTreeMap::new();
        let license_source = if files.is_empty() {
            let Some(notice) = upstream
                .iter()
                .find(|n| n.name == package.name && n.version == package.version)
            else {
                missing.push(format!(
                    "{} {} ({})",
                    package.name,
                    package.version,
                    package.license.as_deref().unwrap_or("no license metadata")
                ));
                continue;
            };
            let vcs: serde_json::Value =
                serde_json::from_slice(&fs::read(directory.join(".cargo_vcs_info.json"))?)?;
            if vcs["git"]["sha1"].as_str() != Some(notice.commit.as_str()) {
                bail!("reviewed upstream notice does not match the packaged VCS revision");
            }
            let bytes = format!("{}\n", notice.text.join("\n")).into_bytes();
            if digest(&bytes) != notice.sha256 {
                bail!("reviewed upstream license digest changed");
            }
            let destination = Path::new("dependencies")
                .join(format!("{}-{}", package.name, package.version))
                .join("LICENSE");
            hashes.insert(portable(&destination), digest(&bytes));
            snapshots.insert(destination, bytes);
            Some(notice.source_url.clone())
        } else {
            None
        };
        for file in files {
            let file = fs::canonicalize(file)?;
            let relative = file
                .strip_prefix(&directory)
                .context("license file escapes its package")?;
            let destination = Path::new("dependencies")
                .join(format!("{}-{}", package.name, package.version))
                .join(relative);
            let bytes = fs::read(&file)?;
            hashes.insert(portable(&destination), digest(&bytes));
            snapshots.insert(destination, bytes);
        }
        dependencies.push(DependencyNotice {
            name: package.name.clone(),
            version: package.version.clone(),
            license: package
                .license
                .clone()
                .context("dependency has no SPDX license metadata")?,
            source: package.source.clone(),
            license_source,
            files: hashes,
        });
    }
    if !missing.is_empty() {
        bail!(
            "archives without packaged license files need exact upstream review: {}",
            missing.join(", ")
        );
    }
    dependencies.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    let sysroot = String::from_utf8(capture("rustc".as_ref(), &["--print", "sysroot"])?)?;
    let docs = Path::new(sysroot.trim())
        .join("share")
        .join("doc")
        .join("rust");
    let report = docs.join("COPYRIGHT-library.html");
    if !report.is_file() {
        bail!("Rust library notices require the pinned rust-docs component; see CONTRIBUTING.md");
    }
    let mut rust_files = vec![report];
    collect_files(&docs.join("licenses"), &mut rust_files)?;
    rust_files.sort();
    let mut rust_library_files = BTreeMap::new();
    for file in rust_files {
        let relative = file.strip_prefix(&docs)?;
        let destination = Path::new("rust").join(relative);
        let bytes = fs::read(file)?;
        rust_library_files.insert(portable(&destination), digest(&bytes));
        snapshots.insert(destination, bytes);
    }
    let rustc = String::from_utf8(capture("rustc".as_ref(), &["--version"])?)?
        .trim()
        .to_string();
    let index = NoticeIndex {
        rustc,
        rust_library_files,
        dependencies,
    };
    let mut index = serde_json::to_vec_pretty(&index)?;
    index.push(b'\n');
    snapshots.insert("runtime-index.json".into(), index);
    Ok(snapshots)
}

pub fn notices(write: bool) -> Result<()> {
    let snapshots = expected_notices()?;
    let directory = Path::new("licenses");
    if write {
        for name in ["dependencies", "rust"] {
            remove_owned_directory(directory.join(name))?;
        }
    }
    for (relative, bytes) in &snapshots {
        let target = directory.join(relative);
        if write {
            fs::create_dir_all(target.parent().context("notice has no parent")?)?;
            fs::write(&target, bytes)?;
        } else if fs::read(&target)
            .with_context(|| format!("missing {}; run make notices", target.display()))?
            != *bytes
        {
            bail!(
                "{} differs from its locked dependency/toolchain; refresh and review notices",
                target.display()
            );
        }
    }
    if !write {
        let mut actual = vec![];
        for name in ["dependencies", "rust"] {
            collect_files(&directory.join(name), &mut actual)?;
        }
        if actual.len() + 1 != snapshots.len() {
            bail!("stale generated dependency notices; run make notices");
        }
    }
    println!(
        "{} locked runtime and Rust-library notice files {}.",
        snapshots.len(),
        if write { "written" } else { "verified" }
    );
    Ok(())
}

pub fn license() -> Result<()> {
    let license = fs::read_to_string("LICENSE")?;
    if !license.contains("Apache License") || !license.contains("Version 2.0") {
        bail!("LICENSE must contain Apache License 2.0");
    }
    if !fs::read_to_string("NOTICE")?.contains("BlackoutSecure contributors") {
        bail!("NOTICE must retain original attribution");
    }
    let mut sources = vec![];
    for directory in ["src", "tests"] {
        collect_files(Path::new(directory), &mut sources)?;
    }
    sources.push("api/v1beta1/event.proto".into());
    for file in sources
        .iter()
        .filter(|p| matches!(p.extension().and_then(|s| s.to_str()), Some("rs" | "proto")))
    {
        let source = fs::read_to_string(file)?;
        if source.lines().next() != Some("// SPDX-License-Identifier: Apache-2.0") {
            bail!("{} lacks its original-code SPDX header", file.display());
        }
    }
    notices(false)?;
    println!("Original Rust/protobuf attribution and locked dependency notices match.");
    Ok(())
}

fn remove_owned_directory(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("refusing to clean non-directory {}", path.display());
    }
    fs::remove_dir_all(path)?;
    Ok(())
}

pub fn clean() -> Result<()> {
    match fs::symlink_metadata("target") {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            bail!("refusing to clean non-directory target")
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
        _ => {}
    }
    for directory in ["bin", ".tools"] {
        remove_owned_directory(directory)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleaning_never_follows_a_file_or_external_symlink() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-data")
            .join(format!("tool-clean-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let target = directory.join("target");
        fs::write(&target, b"must remain").unwrap();
        assert!(remove_owned_directory(&target).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"must remain");
        fs::remove_file(&target).unwrap();
        #[cfg(unix)]
        {
            let external = directory.join("external");
            fs::create_dir(&external).unwrap();
            std::os::unix::fs::symlink(&external, &target).unwrap();
            assert!(remove_owned_directory(&target).is_err());
            assert!(external.is_dir());
        }
        fs::remove_dir_all(directory).unwrap();
    }
}
