//! Implementation of [Open Document Format Version 1.2 - Part 3: Packages](https://www.iso.org/standard/66376.html).

// tayu@mainpc:~/src/rodf/sample (master *%=)$ ls
// ACI-Simulator.odp  Configurations2  content.xml  META-INF  meta.xml  mimetype  Pictures  settings.xml  styles.xml  Thumbnails
// tayu@mainpc:~/src/rodf/sample (master *%=)$ cat mimetype
// application/vnd.oasis.opendocument.presentation
// tayu@mainpc:~/src/rodf/sample (master *%=)$ ls META-INF/
// manifest.xml

use std::{
    io::{Cursor, Read, Seek},
    sync::LazyLock,
};

use anyxml::{
    error::XMLError,
    mediatype::{ApplicationSubtype::RelaxNgCompactSyntax, MediaType, MediaTypeError},
    relaxng::RelaxNGSchema,
    sax::{DefaultSAXHandler, XMLReader},
    tree::TreeBuildHandler,
};
use zip::{ZipArchive, result::ZipError};

use crate::ODF_MANIFEST_NAMESPACE;

#[derive(Debug)]
pub enum PackageError {
    ZipError(ZipError),
    IOError(std::io::Error),
    MediaTypeError(MediaTypeError),
    XMLError(XMLError),
}

impl From<ZipError> for PackageError {
    fn from(value: ZipError) -> Self {
        Self::ZipError(value)
    }
}

impl From<std::io::Error> for PackageError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<MediaTypeError> for PackageError {
    fn from(value: MediaTypeError) -> Self {
        Self::MediaTypeError(value)
    }
}

impl From<XMLError> for PackageError {
    fn from(value: XMLError) -> Self {
        Self::XMLError(value)
    }
}

impl std::fmt::Display for PackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZipError(err) => write!(f, "{}", err),
            Self::IOError(err) => write!(f, "{}", err),
            Self::MediaTypeError(err) => write!(f, "{}", err),
            Self::XMLError(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for PackageError {}

macro_rules! manifest_schema {
    ( $var:ident, $version:literal, $base:literal ) => {
        static $var: LazyLock<RelaxNGSchema> = LazyLock::new(|| {
            let reader = include_bytes!(concat!(
                "../resources/schemas/",
                $version,
                "/",
                $base,
                ".zip"
            ))
            .as_slice();
            let mut archive = ZipArchive::new(Cursor::new(reader)).unwrap();
            let bytes = archive.by_name(concat!($base, ".rnc")).unwrap();
            RelaxNGSchema::parse_compact_reader(bytes, None, None, None::<DefaultSAXHandler>)
                .unwrap()
        });
    };
}
manifest_schema!(
    MANIFEST_SCHEMA_V10,
    "v1.0",
    "OpenDocument-manifest-schema-v1.0-os"
);
manifest_schema!(
    MANIFEST_SCHEMA_V11,
    "v1.1",
    "OpenDocument-manifest-schema-v1.1"
);
manifest_schema!(
    MANIFEST_SCHEMA_V12,
    "v1.2",
    "OpenDocument-v1.2-os-manifest-schema"
);
manifest_schema!(
    MANIFEST_SCHEMA_V13,
    "v1.3",
    "OpenDocument-v1.3-manifest-schema"
);
manifest_schema!(
    MANIFEST_SCHEMA_V14,
    "v1.4",
    "OpenDocument-v1.4-manifest-schema"
);

pub struct Package<R: Read + Seek> {
    mimetype: Option<MediaType>,
    manifest: (),
    archive: ZipArchive<R>,
}

impl<R: Read + Seek> Package<R> {
    pub fn from_reader(reader: R) -> Result<Self, PackageError> {
        let mut archive = ZipArchive::new(reader)?;
        let mut mimetype = None;
        if let Ok(mut file) = archive.by_name("mimetype") {
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            mimetype = Some(buf.parse()?);
        }

        let manifest_file = archive.by_name("META-INF/manifest.xml")?;
        let mut reader = XMLReader::builder()
            .set_handler(TreeBuildHandler::default())
            .build();
        reader.parse_reader(manifest_file, None, None)?;
        if reader.handler.fatal_error {
            return Err(PackageError::XMLError(XMLError::InternalError));
        }
        let document = reader.handler.document.clone();
        let manifest_version = document
            .document_element()
            .and_then(|root| root.get_attribute("version", Some(ODF_MANIFEST_NAMESPACE)));
        // let schema = match manifest_version.as_deref() {
        //     Some("1.2") => MANIFEST_SCHEMA_V12.clone(),
        //     Some("1.3") => MANIFEST_SCHEMA_V13.clone(),
        //     Some("1.4") => MANIFEST_SCHEMA_V14.clone(),
        //     _ => {
        //         // Since there is no way to distinguish between 1.1 and 1.0, assume it is 1.1.
        //         // Also, even if a version is found, if it is unsupported, fall back to 1.1.
        //         MANIFEST_SCHEMA_V11.clone()
        //     }
        // };

        drop(reader);
        Ok(Self {
            mimetype,
            manifest: (),
            archive,
        })
    }
}
