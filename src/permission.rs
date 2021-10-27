use std::{
    collections::HashSet,
    hash::Hash,
    path::{Path, PathBuf},
};

use crate::Error;

/// Represents whether permission is granted or denied.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PermissionState {
    Granted = 0,
    Denied = 1,
}

impl Default for PermissionState {
    fn default() -> Self {
        PermissionState::Denied
    }
}

impl PermissionState {
    fn fmt_access(name: &str, info: Option<&str>) -> String {
        format!(
            "{} access{}",
            name,
            info.map_or(String::new(), |info| { format!(" to {}", info) }),
        )
    }

    fn error(name: &str, info: Option<&str>) -> Error {
        Error::PermissionDeniedError(format!(
            "Requires {}, run again with --allow-{} flag",
            Self::fmt_access(name, info),
            name
        ))
    }

    pub fn check(self, name: &str, info: Option<&str>) -> Result<(), Error> {
        match self {
            PermissionState::Granted => Ok(()),
            _ => Err(Self::error(name, info)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitPermission {
    pub name: &'static str,
    pub state: PermissionState,
}

impl UnitPermission {
    pub fn check(&self) -> Result<(), Error> {
        self.state.check(self.name, None)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryPermission<T: Eq + Hash> {
    pub name: &'static str,
    pub global_state: PermissionState,
    pub granted_list: HashSet<T>,
    pub denied_list: HashSet<T>,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct ReadDescriptor(pub PathBuf);

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct WriteDescriptor(pub PathBuf);

impl UnaryPermission<ReadDescriptor> {
    pub fn check(&self, path: &Path) -> Result<(), Error> {
        if self.global_state == PermissionState::Granted {
            if self
                .denied_list
                .iter()
                .any(|path_| path_.0.starts_with(path))
            {
                PermissionState::Denied.check(self.name, path.to_str())
            } else {
                PermissionState::Granted.check(self.name, path.to_str())
            }
        } else if self
            .granted_list
            .iter()
            .any(|path_| path.starts_with(&path_.0))
        {
            PermissionState::Granted.check(self.name, path.to_str())
        } else {
            PermissionState::Denied.check(self.name, path.to_str())
        }
    }
}

impl UnaryPermission<WriteDescriptor> {
    pub fn check(&self, path: &Path) -> Result<(), Error> {
        if self.global_state == PermissionState::Granted {
            if self
                .denied_list
                .iter()
                .any(|path_| path_.0.starts_with(path))
            {
                PermissionState::Denied.check(self.name, path.to_str())
            } else {
                PermissionState::Granted.check(self.name, path.to_str())
            }
        } else if self
            .granted_list
            .iter()
            .any(|path_| path.starts_with(&path_.0))
        {
            PermissionState::Granted.check(self.name, path.to_str())
        } else {
            PermissionState::Denied.check(self.name, path.to_str())
        }
    }
}

impl Default for UnaryPermission<ReadDescriptor> {
    fn default() -> Self {
        UnaryPermission::<ReadDescriptor> {
            name: "read",
            global_state: Default::default(),
            granted_list: Default::default(),
            denied_list: Default::default(),
        }
    }
}

impl Default for UnaryPermission<WriteDescriptor> {
    fn default() -> Self {
        UnaryPermission::<WriteDescriptor> {
            name: "write",
            global_state: Default::default(),
            granted_list: Default::default(),
            denied_list: Default::default(),
        }
    }
}

/// A simple permission manager.
#[derive(Clone, Debug, PartialEq)]
pub struct Permissions {
    pub read: UnaryPermission<ReadDescriptor>,
    pub write: UnaryPermission<WriteDescriptor>,
    pub net: UnitPermission,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            read: UnaryPermission {
                global_state: PermissionState::Granted,
                ..Default::default()
            },
            write: UnaryPermission {
                global_state: PermissionState::Denied,
                ..Default::default()
            },
            net: UnitPermission {
                name: "net",
                state: PermissionState::Granted,
            },
        }
    }
}
