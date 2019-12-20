#![deny(clippy::all)]

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::Status;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
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
    pub name: Option<String>,
    pub needs: Option<Vec<String>>,
    pub force: Option<bool>,
    pub path: PathBuf,
    pub src: Option<PathBuf>,
    pub state: FileState,
}
impl Default for File {
    fn default() -> Self {
        Self {
            name: None,
            needs: None,
            force: None,
            path: PathBuf::new(),
            src: None,
            state: FileState::Touch,
        }
    }
}
impl File {
    pub fn execute(&self) -> super::Result {
        match self.state {
            FileState::Absent => execute_absent(&self.path),
            FileState::Directory => execute_directory(&self.path, self.force.unwrap_or(false)),
            FileState::Link => match &self.src {
                Some(s) => execute_link(s, &self.path, self.force.unwrap_or(false)),
                None => Err(super::Error::Other(String::from("state=link requires src"))),
            },
            FileState::Touch => execute_touch(&self.path),
            _ => Err(super::Error::Other(format!(
                "state={} not implemented",
                format!("{:?}", &self.state).to_lowercase(),
            ))),
        }
    }
}

fn execute_absent<P>(path: P) -> super::Result
where
    P: AsRef<Path>,
{
    let p = path.as_ref();
    if !p.exists() {
        return Ok(Status::NoChange(format!("{}", p.display())));
    }

    if p.is_dir() {
        fs::remove_dir_all(&p)?;
    } else {
        fs::remove_file(&p)?;
    }
    Ok(Status::Changed(
        format!("{}", p.display()),
        String::from("absent"),
    ))
}

fn execute_directory<P>(path: P, force: bool) -> super::Result
where
    P: AsRef<Path>,
{
    let p = path.as_ref();
    let previously;
    if p.is_dir() {
        return Ok(Status::NoChange(format!("directory: {}", p.display())));
    } else if p.exists() {
        if !force {
            return Err(super::Error::Other(format!("exists: {}", p.display())));
        }
        previously = String::from("not directory");
        execute_absent(&p)?;
    } else {
        previously = String::from("absent");
    }

    fs::create_dir_all(&p)?;
    Ok(Status::Changed(
        previously,
        format!("directory: {}", p.display()),
    ))
}

fn execute_link<P>(src: P, dest: P, force: bool) -> super::Result
where
    P: AsRef<Path>,
{
    let s = src.as_ref();
    if std::fs::symlink_metadata(&s).is_err() && !force {
        return Err(super::Error::Other(format!("absent src: {}", &s.display())));
    }

    let d = dest.as_ref();
    let mut previously = String::from("absent");

    if let Ok(target) = std::fs::read_link(&d) {
        previously = format!("{} -> {}", target.display(), d.display());
        if s == target {
            return Ok(Status::NoChange(previously));
        }
        if !force {
            return Err(super::Error::Other(format!(
                "existing link: {}",
                previously
            )));
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
                return Err(super::Error::Other(previously));
            }
        }
        Err(_) => {
            if let Some(parent) = d.parent() {
                execute_directory(&parent, force)?;
            }
        }
    }

    symbolic_link(&s, &d)?;

    Ok(Status::Changed(
        previously,
        format!("{} -> {}", s.display(), d.display(),),
    ))
}

fn execute_touch<P>(path: P) -> super::Result
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
    fs::write(p, "")?;
    Ok(Status::Changed(
        String::from("absent"),
        format!("{}", p.display()),
    ))
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

    use crate::jobs::Error;

    use super::*;

    #[test]
    fn absent_deletes_existing_file() -> Result<(), Error> {
        let file = File {
            path: Temp::new_file()?.to_path_buf(),
            state: FileState::Absent,
            ..Default::default()
        };

        fs::create_dir_all(&file.path.parent().unwrap())?;
        fs::write(&file.path, "")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(format!("{}", file.path.display()), String::from("absent"))
        );
        assert!(fs::symlink_metadata(&file.path).is_err());
        Ok(())
    }

    #[test]
    fn absent_deletes_existing_directory() -> Result<(), Error> {
        let file = File {
            path: Temp::new_dir()?.to_path_buf(),
            state: FileState::Absent,
            ..Default::default()
        };

        fs::create_dir_all(&file.path)?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(format!("{}", file.path.display()), String::from("absent"))
        );
        assert!(fs::symlink_metadata(&file.path).is_err());
        Ok(())
    }

    #[test]
    fn absent_makes_nochange_when_already_absent() -> Result<(), Error> {
        let file = File {
            path: Temp::new_dir()?.join("missing.txt"),
            state: FileState::Absent,
            ..Default::default()
        };

        let got = file.execute()?;

        assert_eq!(got, Status::NoChange(format!("{}", file.path.display())));
        Ok(())
    }

    #[test]
    fn link_symlinks_src_to_path() -> Result<(), Error> {
        let src = Temp::new_file()?.to_path_buf();
        let file = File {
            path: Temp::new_file()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        fs::write(&src, "hello")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                String::from("absent"),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs::read_to_string(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_symlinks_src_to_path_in_new_directory() -> Result<(), Error> {
        let src = Temp::new_file()?.to_path_buf();
        let file = File {
            path: Temp::new_dir()?.join("symlink.txt"),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        fs::create_dir_all(file.path.parent().unwrap())?;
        fs::write(&src, "hello")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                String::from("absent"),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs::read_to_string(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_corrects_existing_symlink() -> Result<(), Error> {
        let src_old = Temp::new_file()?.to_path_buf();
        let file_old = File {
            path: Temp::new_dir()?.join("symlink.txt"),
            src: Some(src_old.clone()),
            state: FileState::Link,
            ..Default::default()
        };
        fs::write(&src_old, "hello_old")?;
        file_old.execute()?;

        let src = Temp::new_file()?.to_path_buf();
        let file = File {
            force: Some(true),
            path: file_old.path,
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        fs::write(&src, "hello")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                format!("{} -> {}", &src_old.display(), file.path.display()),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs::read_to_string(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_removes_existing_file_at_path() -> Result<(), Error> {
        let src = Temp::new_file()?.to_path_buf();
        let file = File {
            force: Some(true),
            path: Temp::new_file()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        fs::write(&src, "hello")?;
        fs::write(&file.path, "existing")?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                format!("existing: {}", file.path.display()),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs::read_to_string(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_removes_existing_directory_at_path() -> Result<(), Error> {
        let src = Temp::new_file()?.to_path_buf();
        let file = File {
            force: Some(true),
            path: Temp::new_dir()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        fs::write(&src, "hello")?;
        fs::create_dir_all(&file.path)?;
        let got = file.execute()?;

        assert_eq!(
            got,
            Status::Changed(
                format!("existing: {}", file.path.display()),
                format!("{} -> {}", &src.display(), file.path.display())
            )
        );
        assert_eq!(fs::read_to_string(&file.path)?, "hello");
        Ok(())
    }

    #[test]
    fn link_without_force_requires_src_to_exist() -> Result<(), Error> {
        let src = Temp::new_file()?.to_path_buf();
        let file = File {
            path: Temp::new_dir()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        let got = file.execute();

        assert!(got.is_err());
        assert_eq!(
            got.err().unwrap(),
            Error::Other(format!("absent src: {}", src.display()))
        );
        Ok(())
    }

    #[test]
    fn link_without_force_requires_path_to_not_exist() -> Result<(), Error> {
        let src = Temp::new_file()?.to_path_buf();
        let file = File {
            path: Temp::new_dir()?.to_path_buf(),
            src: Some(src.clone()),
            state: FileState::Link,
            ..Default::default()
        };

        fs::write(&src, "hello")?;
        fs::create_dir_all(&file.path)?;
        let got = file.execute();

        assert!(got.is_err());
        assert_eq!(
            got.err().unwrap(),
            Error::Other(format!("existing: {}", file.path.display()))
        );
        Ok(())
    }

    #[test]
    fn touch_creates_new_empty_file() -> Result<(), Error> {
        let file = File {
            path: Temp::new_dir()?.join("new.txt"),
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
    fn touch_makes_nochange_for_existing_path() -> Result<(), Error> {
        let file = File {
            path: Temp::new_file()?.to_path_buf(),
            state: FileState::Touch,
            ..Default::default()
        };

        fs::create_dir_all(file.path.parent().unwrap())?;
        fs::write(&file.path, "")?;
        let got = file.execute()?;

        assert_eq!(got, Status::NoChange(format!("{}", file.path.display())));
        Ok(())
    }
}
