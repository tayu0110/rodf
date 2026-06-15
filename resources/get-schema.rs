#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2024"
[dependencies]
tokio = { version = "=1.52.3", features = ["full"] }
reqwest = { version = "=0.13.3" }
zip = { version = "8.6.0", features = ["deflate"] }
---

use std::{fs::File, io::Write as _, process::Stdio};

use tokio::{process::Command, task::JoinSet};
use zip::{ZipWriter, write::SimpleFileOptions};

// https://www.oasis-open.org/standards/
const V1_0_SCHEMAS: &[&str] = &[
    "https://groups.oasis-open.org/higherlogic/ws/public/download/12571/OpenDocument-schema-v1.0-os.rng",
    "https://groups.oasis-open.org/higherlogic/ws/public/download/12570/OpenDocument-manifest-schema-v1.0-os.rng",
    "https://groups.oasis-open.org/higherlogic/ws/public/download/12569/OpenDocument-strict-schema-v1.0-os.rng",
];
const V1_1_SCHEMAS: &[&str] = &[
    "https://docs.oasis-open.org/office/v1.1/errata01/os/OpenDocument-manifest-schema-v1.1.rng",
    "https://docs.oasis-open.org/office/v1.1/errata01/os/OpenDocument-schema-v1.1-errata01-complete.rng",
    "https://docs.oasis-open.org/office/v1.1/errata01/os/OpenDocument-strict-schema-v1.1-errata01-complete.rng",
];
const V1_2_SCHEMAS: &[&str] = &[
    "https://docs.oasis-open.org/office/v1.2/os/OpenDocument-v1.2-os-dsig-schema.rng",
    "https://docs.oasis-open.org/office/v1.2/os/OpenDocument-v1.2-os-manifest-schema.rng",
    "https://docs.oasis-open.org/office/v1.2/os/OpenDocument-v1.2-os-metadata.owl",
    "https://docs.oasis-open.org/office/v1.2/os/OpenDocument-v1.2-os-package-metadata.owl",
    "https://docs.oasis-open.org/office/v1.2/os/OpenDocument-v1.2-os-schema.rng",
];
const V1_3_SCHEMAS: &[&str] = &[
    "https://docs.oasis-open.org/office/OpenDocument/v1.3/os/schemas/OpenDocument-v1.3-dsig-schema.rng",
    "https://docs.oasis-open.org/office/OpenDocument/v1.3/os/schemas/OpenDocument-v1.3-manifest-schema.rng",
    "https://docs.oasis-open.org/office/OpenDocument/v1.3/os/schemas/OpenDocument-v1.3-metadata.owl",
    "https://docs.oasis-open.org/office/OpenDocument/v1.3/os/schemas/OpenDocument-v1.3-package-metadata.owl",
    "https://docs.oasis-open.org/office/OpenDocument/v1.3/os/schemas/OpenDocument-v1.3-schema.rng",
];
const V1_4_SCHEMAS: &[&str] = &[
    "https://docs.oasis-open.org/office/OpenDocument/v1.4/os/schemas/OpenDocument-v1.4-dsig-schema.rng",
    "https://docs.oasis-open.org/office/OpenDocument/v1.4/os/schemas/OpenDocument-v1.4-manifest-schema.rng",
    "https://docs.oasis-open.org/office/OpenDocument/v1.4/os/schemas/OpenDocument-v1.4-metadata.owl",
    "https://docs.oasis-open.org/office/OpenDocument/v1.4/os/schemas/OpenDocument-v1.4-package-metadata.owl",
    "https://docs.oasis-open.org/office/OpenDocument/v1.4/os/schemas/OpenDocument-v1.4-schema.rng",
];

async fn get_schema(
    version: &str,
    urls: &'static [&'static str],
) -> Result<(), Box<dyn std::error::Error>> {
    let base_dir = format!("schemas/{version}");
    let mut set = JoinSet::new();
    for url in urls {
        let filename = url.rsplit_once('/').unwrap().1;
        set.spawn(async move {
            (async move || -> Result<(String, &str), reqwest::Error> {
                let client = reqwest::Client::builder().build()?;
                Ok((client.get(*url).send().await?.text().await?, filename))
            })()
            .await
        });
    }
    tokio::fs::remove_dir_all(&base_dir).await.ok();
    tokio::fs::create_dir_all(&base_dir).await.ok();
    let mut set2 = JoinSet::new();
    while let Some((text, filename)) = set.join_next().await.transpose()?.transpose()? {
        let base_dir = base_dir.clone();
        set2.spawn(async move {
            (async move || -> Result<String, std::io::Error> {
                let filename = format!("{base_dir}/{filename}");
                tokio::fs::write(&filename, text).await?;
                Ok(filename)
            })()
            .await
        });
    }
    let ret = set2.join_all().await;
    let mut set3 = JoinSet::new();
    for filename in ret {
        let filename = filename?;
        if filename.ends_with(".rng") {
            set3.spawn(async move {
                (async move || -> Result<(String, String), std::io::Error> {
                    let rnc = filename.replace(".rng", ".rnc");
                    let ret = Command::new("trang")
                        .arg("-I")
                        .arg("rng")
                        .arg("-O")
                        .arg("rnc")
                        .arg(filename)
                        .arg("/dev/stdout")
                        .stdout(Stdio::piped())
                        .output()
                        .await?;
                    Ok((String::from_utf8_lossy(&ret.stdout).into_owned(), rnc))
                })()
                .await
            });
        }
    }
    let ret = set3.join_all().await;
    for ret in ret {
        let (ret, rnc) = ret?;
        let zip = rnc.replace(".rnc", ".zip");
        let file = File::options().write(true).create(true).open(zip)?;
        let mut archive = ZipWriter::new(file);
        archive.start_file(rnc, SimpleFileOptions::default())?;
        archive.write_all(ret.as_bytes())?;
        archive.finish()?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio::try_join!(
        get_schema("v1.0", V1_0_SCHEMAS),
        get_schema("v1.1", V1_1_SCHEMAS),
        get_schema("v1.2", V1_2_SCHEMAS),
        get_schema("v1.3", V1_3_SCHEMAS),
        get_schema("v1.4", V1_4_SCHEMAS),
    )?;
    Ok(())
}
