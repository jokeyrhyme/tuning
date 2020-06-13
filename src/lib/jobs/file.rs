use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

use super::Status;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("unable to link {}->{}: {}", src.display(), path.display(), source)]
    CreateLink {
        path: PathBuf,
        src: PathBuf,
        source: io::Error,
    },
    #[error("unable to create {}: {}", path.display(), source)]
    CreatePath { path: PathBuf, source: io::Error },
    #[error("{} already exists", path.display())]
    PathExists { path: PathBuf },
    #[allow(dead_code)] // TODO: test-only errors should not be here
    #[error("unable to read {}: {}", path.display(), source)]
    ReadPath { path: PathBuf, source: io::Error },
    #[error("unable to remove {}: {}", path.display(), source)]
    RemovePath { path: PathBuf, source: io::Error },
    #[error("{} not found", src.display())]
    SrcNotFound { src: PathBuf },
    #[error("state={} requires src", format!("{:?}", state).to_lowercase())]
    StateRequiresSrc { state: FileState },
    #[error("state={} is not yet implemented", format!("{:?}", state).to_lowercase())]
    StateNotImplemented { state: FileState },
    #[allow(dead_code)] // TODO: test-only errors should not be here
    #[error(transparent)]
    TempPath { source: io::Error },
    #[error("unable to write {}: {}", path.display(), source)]
    WritePath { path: PathBuf, source: io::Error },
}
impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileState {
    Absent,
    Directory,
    File,
    Hard,
    Link,
    Touch,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub struct File {
    pub force: Option<bool>,
    pub path: PathBuf,
    pub src: Option<PathBuf>,
    pub state: FileState,
}
impl Default for File {
    fn default() -> Self {
        Self {
            force: None,
            path: PathBuf::new(),
            src: None,
            state: FileState::Touch,
        }
    }
}
impl File {
    pub fn execute(&self) -> Result {
        match self.state {
            FileState::Absent => execute_absent(&self.path),
            FileState::Directory => execute_directory(&self.path, self.force.unwrap_or(false)),
            FileState::Link => match &self.src {
                Some(s) => execute_link(s, &self.path, self.force.unwrap_or(false)),
                None => Err(Error::StateRequiresSrc { state: self.state }),
            },
            FileState::Touch => execute_touch(&self.path),
            _ => Err(Error::StateNotImplemented { state: self.state }),
        }
    }

    pub fn name(&self) -> String {
        let force = self.force.unwrap_or(false);
        let pd = self.path.display();
        match self.state {
            FileState::Absent => format!("rm -r{} {}", if force { "f" } else { "" }, pd),
            FileState::Directory => format!("mkdir -p {}", pd),
            FileState::Link => format!(
                "ln -s{} {} {}",
                if force { "f" } else { "" },
                self.src.clone().unwrap_or_default().display(),
                pd
            ),
            FileState::Touch => format!("touch {}", pd),
            _ => format!("{:#?}", self),
        }
    }
}

pub type Result = std::result::Result<Status, Error>;

fn execute_absent<P>(path: P) -> Result
where
    P: AsRef<Path>,
{
    let p = path.as_ref();
    if !p.exists() {
        return Ok(Status::NoChange(format!("{}", p.display())));
    }

    (if p.is_dir() {
        fs::remove_dir_all(&p)
    } else {
        fs::remove_file(&p)
    })
    .map_err(|e| Error::RemovePath {
        path: p.to_path_buf(),
        source: e,
    })?;
    Ok(Status::Changed(
        format!("{}", p.display()),
        String::from("absent"),
    ))
}

fn execute_directory<P>(path: P, force: bool) -> Result
where
    P: AsRef<Path>,
{
    let p = path.as_ref();
    let previously;
    if p.is_dir() {
        return Ok(Status::NoChange(format!("directory: {}", p.display())));
    } else if p.exists() {
        if !force {
            return Err(Error::PathExists {
                path: p.to_path_buf(),
            });
        }
        previously = String::from("not directory");
        execute_absent(&p)?;
    } else {
        previously = String::from("absent");
    }

    fs_create_dir_all(&p)?;
    Ok(Status::Changed(
        previously,
        format!("directory: {}", p.display()),
    ))
}

fn execute_link<P>(src: P, dest: P, force: bool) -> Result
where
    P: AsRef<Path>,
{
    let s = src.as_ref();
    if std::fs::symlink_metadata(&s).is_err() && !force {
        return Err(Error::SrcNotFound {
            src: s.to_path_buf(),
        });
    }

    let d = dest.as_ref();
    let mut previously = String::from("absent");

    if let Ok(target) = std::fs::read_link(&d) {
        previously = format!("{} -> {}", target.display(), d.display());
        if s == target {
            return Ok(Status::NoChange(previously));
        }
        if !force {
            return Err(Error::PathExists {
                path: d.to_path_buf(),
            });
        }
    };
    // dest does not exist, or is wrong symlink, or is not a symlink

    match std::fs::symlink_metadata(&d) {
        Ok(attr) => {
            if !attr.file_type().is_symlink() {
                previously = format!("existing: {}", &d.display());
            }
            if force {
                execute_absent(&d)?;
            } else {
                return Err(Error::PathExists {
                    path: d.to_path_buf(),
                });
            }
        }
        Err(_) => {
            if let Some(parent) = d.parent() {
                execute_directory(&parent, force)?;
            }
        }
    }

    symbolic_link(&s, &d).map_err(|e| Error::CreateLink {
        path: d.to_path_buf(),
        src: s.to_path_buf(),
        source: e,
    })?;

    Ok(Status::Changed(
        previously,
        format!("{} -> {}", s.display(), d.display(),),
    ))
}

fn execute_touch<P>(path: P) -> Result
where
    P: AsRef<Path>,
{
    let p = path.as_ref();
    if p.exists() {
        // TODO: consider bumping access/modify time like real `touch`
        return Ok(Status::NoChange(format!("{}", p.display())));
    }
    if let Some(parent) = p.parent() {
        execute_directory(&parent, false)?;
    }
    fs_write(p, "")?;
    Ok(Status::Changed(
        String::from("absent"),
        format!("{}", p.display()),
    ))
}

fn fs_create_dir_all<P>(p: P) -> std::result::Result<(), Error>
where
    P: AsRef<Path>,
{
    fs::create_dir_all(&p).map_err(|e| Error::CreatePath {
        path: p.as_ref().to_path_buf(),
        source: e,
    })
}

fn fs_write<P, C>(p: P, c: C) -> std::result::Result<(), Error>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    fs::write(&p, c).map_err(|e| Error::WritePath {
        path: p.as_ref().to_path_buf(),
        source: e,
    })
}

#[cfg(not(windows))]
fn symbolic_link<P>(src: P, dest: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    std::os::unix::fs::symlink(src.as_ref(), dest.as_ref())
}

#[cfg(windows)]
fn symbolic_link<P>(src: P, dest: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let src_attr = std::fs::symlink_metadata(&src)?;
    if src_attr.is_dir() {
        return std::os::windows::fs::symlink_dir(&src, dest);
    }

    std::os::windows::fs::symlink_file(&src, dest)
}

#[cfg(test)]
mod tests {
    use mktemp::Temp;

    use super::*;

    #[test]
    fn absent_deletes_existing_file() -> std::result::Result<(), Error> {
        let file = File {
            path: temp_file()?.to_path_buf(),
            state: FileState::Absent,
            ..Default::default()
        };

        fs_create_dir_all(&file.path.parent().unwrap())?;
        fs_write(&file.path, "")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(format!("{}", file.path.display()), String::from("absent"))
        );
        assert!(fs::symlink_metadata(&file.path).is_err());
        Ok(())
    }

    #[test]
    fn absent_deletes_existing_directory() -> std::result::Result<(), Error> {
        let file = File {
            path: temp_dir()?.to_path_buf(),
            state: FileState::Absent,
            ..Default::default()
        };

        fs_create_dir_all(&file.path)?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(format!("{}", file.path.display()), String::from("absent"))
        );
        assert!(fs::symlink_metadata(&file.path).is_err());
        Ok(())
    }

    #[test]
    fn absent_makes_nochange_when_already_absent() -> std::result::Result<(), Error> {
        let file = File {
            path: temp_dir()?.join("missing.txt"),
            state: FileState::Absent,
            ..Default::default()
        };

        let got = file.execute()?;

        assert_eq!(got, Status::NoChange(format!("{}", file.path.display())));
        Ok(())
    }

    #[test]
    fn link_symlinks_src_to_path() -> std::result::Result<(), Error> {
        let src = temp_file()?.to_path_buf();
        let file = File {
            path: temp_file()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        fs_write(&src, "hello")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                String::from("absent"),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs_read(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_symlinks_src_to_path_in_new_directory() -> std::result::Result<(), Error> {
        let src = temp_file()?.to_path_buf();
        let file = File {
            path: temp_dir()?.join("symlink.txt"),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        fs_create_dir_all(file.path.parent().unwrap())?;
        fs_write(&src, "hello")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                String::from("absent"),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs_read(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_corrects_existing_symlink() -> std::result::Result<(), Error> {
        let src_old = temp_file()?.to_path_buf();
        let file_old = File {
            path: temp_dir()?.join("symlink.txt"),
            src: Some(src_old.clone()),
            state: FileState::Link,
            ..Default::default()
        };
        fs_write(&src_old, "hello_old")?;
        file_old.execute()?;

        let src = temp_file()?.to_path_buf();
        let file = File {
            force: Some(true),
            path: file_old.path,
            src: Some(src.clone()),
            state: FileState::Link,
        };

        fs_write(&src, "hello")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                format!("{} -> {}", &src_old.display(), file.path.display()),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs_read(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_removes_existing_file_at_path() -> std::result::Result<(), Error> {
        let src = temp_file()?.to_path_buf();
        let file = File {
            force: Some(true),
            path: temp_file()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
        };

        fs_write(&src, "hello")?;
        fs_write(&file.path, "existing")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                format!("existing: {}", file.path.display()),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs_read(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_removes_existing_directory_at_path() -> std::result::Result<(), Error> {
        let src = temp_file()?.to_path_buf();
        let file = File {
            force: Some(true),
            path: temp_dir()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
        };

        fs_write(&src, "hello")?;
        fs_create_dir_all(&file.path)?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                format!("existing: {}", file.path.display()),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs_read(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_without_force_requires_src_to_exist() -> std::result::Result<(), Error> {
        let src = temp_file()?.to_path_buf();
        let file = File {
            path: temp_dir()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        let got = file.execute();

        assert!(got.is_err());
        assert_eq!(got.err().unwrap(), Error::SrcNotFound { src },);
        Ok(())
    }

    #[test]
    fn link_without_force_requires_path_to_not_exist() -> std::result::Result<(), Error> {
        let src = temp_file()?.to_path_buf();
        let file = File {
            path: temp_dir()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        fs_write(&src, "hello")?;
        fs_create_dir_all(&file.path)?;
        let got = file.execute();

        assert!(got.is_err());
        assert_eq!(got.err().unwrap(), Error::PathExists { path: file.path },);
        Ok(())
    }

    #[test]
    fn name_absent() {
        let file = File {
            path: PathBuf::from("foo"),
            state: FileState::Absent,
            ..Default::default()
        };
        let got = file.name();
        let want = "rm -r foo";
        assert_eq!(got, want);
    }

    #[test]
    fn name_absent_force() {
        let file = File {
            force: Some(true),
            path: PathBuf::from("foo"),
            state: FileState::Absent,
            ..Default::default()
        };
        let got = file.name();
        let want = "rm -rf foo";
        assert_eq!(got, want);
    }

    #[test]
    fn name_directory() {
        let file = File {
            path: PathBuf::from("foo"),
            state: FileState::Directory,
            ..Default::default()
        };
        let got = file.name();
        let want = "mkdir -p foo";
        assert_eq!(got, want);
    }

    #[test]
    fn name_link() {
        let file = File {
            path: PathBuf::from("foo"),
            src: Some(PathBuf::from("bar")),
            state: FileState::Link,
            ..Default::default()
        };
        let got = file.name();
        let want = "ln -s bar foo";
        assert_eq!(got, want);
    }

    #[test]
    fn name_link_force() {
        let file = File {
            force: Some(true),
            path: PathBuf::from("foo"),
            src: Some(PathBuf::from("bar")),
            state: FileState::Link,
        };
        let got = file.name();
        let want = "ln -sf bar foo";
        assert_eq!(got, want);
    }

    #[test]
    fn name_touch() {
        let file = File {
            path: PathBuf::from("foo"),
            state: FileState::Touch,
            ..Default::default()
        };
        let got = file.name();
        let want = "touch foo";
        assert_eq!(got, want);
    }

    #[test]
    fn touch_creates_new_empty_file() -> std::result::Result<(), Error> {
        let file = File {
            path: temp_dir()?.join("new.txt"),
            state: FileState::Touch,
            ..Default::default()
        };

        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(String::from("absent"), format!("{}", file.path.display()))
        );
        Ok(())
    }

    #[test]
    fn touch_makes_nochange_for_existing_path() -> std::result::Result<(), Error> {
        let file = File {
            path: temp_file()?.to_path_buf(),
            state: FileState::Touch,
            ..Default::default()
        };

        fs_create_dir_all(file.path.parent().unwrap())?;
        fs_write(&file.path, "")?;
        let got = file.execute()?;

        assert_eq!(got, Status::NoChange(format!("{}", file.path.display())));
        Ok(())
    }

    fn fs_read<P>(p: P) -> std::result::Result<String, Error>
    where
        P: AsRef<Path>,
    {
        let pb = p.as_ref().to_path_buf();
        fs::read_to_string(&pb).map_err(|e| Error::ReadPath {
            path: pb,
            source: e,
        })
    }
    fn temp_dir() -> std::result::Result<mktemp::Temp, Error> {
        Temp::new_dir().map_err(|e| Error::TempPath { source: e })
    }
    fn temp_file() -> std::result::Result<mktemp::Temp, Error> {
        Temp::new_file().map_err(|e| Error::TempPath { source: e })
    }
}
