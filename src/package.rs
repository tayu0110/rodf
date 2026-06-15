//! Implementation of [Open Document Format Version 1.2 - Part 3: Packages](https://www.iso.org/standard/66376.html).

// tayu@mainpc:~/src/rodf/sample (master *%=)$ ls
// ACI-Simulator.odp  Configurations2  content.xml  META-INF  meta.xml  mimetype  Pictures  settings.xml  styles.xml  Thumbnails
// tayu@mainpc:~/src/rodf/sample (master *%=)$ cat mimetype
// application/vnd.oasis.opendocument.presentation
// tayu@mainpc:~/src/rodf/sample (master *%=)$ ls META-INF/
// manifest.xml

use std::io::{Read, Seek};

use anyxml::mediatype::{MediaType, MediaTypeError};
use zip::{ZipArchive, result::ZipError};

#[derive(Debug)]
pub enum PackageError {
    ZipError(ZipError),
    IOError(std::io::Error),
    MediaTypeError(MediaTypeError),
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

impl std::fmt::Display for PackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZipError(err) => write!(f, "{}", err),
            Self::IOError(err) => write!(f, "{}", err),
            Self::MediaTypeError(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for PackageError {}

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

        let mut manifest_file = archive.by_name("META-INF/manifest.xml")?;

        drop(manifest_file);
        Ok(Self {
            mimetype,
            manifest: (),
            archive,
        })
    }
}
