use std::fmt;

use crate::domain::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Owner {
    Self_,
    User,
    World,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Mode {
    Observed,
    Said,
    Acted,
    Inferred,
    Draft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventKind {
    Observation,
    Conversation,
    Action,
    Reflection,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct Namespace(String);

impl Namespace {
    pub fn self_() -> Self {
        Self("self".to_string())
    }

    pub fn world() -> Self {
        Self("world".to_string())
    }

    pub fn for_user(user_id: impl AsRef<str>) -> Self {
        Self(format!("user/{}", user_id.as_ref()))
    }

    pub fn for_project(project_id: impl AsRef<str>) -> Self {
        Self(format!("project/{}", project_id.as_ref()))
    }

    pub fn for_owner(owner: Owner) -> Self {
        match owner {
            Owner::Self_ => Self::self_(),
            Owner::User => Self::for_user("default"),
            Owner::World | Owner::Unknown => Self::world(),
        }
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let is_valid = value == "self"
            || value == "world"
            || value
                .strip_prefix("user/")
                .is_some_and(|suffix| !suffix.is_empty())
            || value
                .strip_prefix("project/")
                .is_some_and(|suffix| !suffix.is_empty());

        if is_valid {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidNamespace)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn derived_owner(&self) -> Owner {
        MemoryScope::for_namespace(self.clone())
            .owner()
            .expect("namespace constructors only permit known namespace shapes")
    }

    pub fn matches_owner(&self, owner: Owner) -> bool {
        match owner {
            Owner::Self_ => self.as_str() == "self",
            Owner::User => self.as_str().starts_with("user/"),
            Owner::World | Owner::Unknown => {
                self.as_str() == "world" || self.as_str().starts_with("project/")
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::parse(value).map_err(|_| serde::de::Error::custom("invalid namespace"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MemoryScope {
    owner: Option<Owner>,
    namespace: Option<Namespace>,
}

impl Owner {
    pub fn is_accepted_for_new_writes(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

impl MemoryScope {
    pub fn for_namespace(namespace: Namespace) -> Self {
        let owner = match namespace.as_str() {
            "self" => Owner::Self_,
            value if value.starts_with("user/") => Owner::User,
            "world" => Owner::World,
            value if value.starts_with("project/") => Owner::World,
            _ => unreachable!("Namespace constructors only permit known namespace shapes"),
        };

        Self {
            owner: Some(owner),
            namespace: Some(namespace),
        }
    }

    pub fn legacy_unscoped() -> Self {
        Self {
            owner: None,
            namespace: None,
        }
    }

    pub fn self_() -> Self {
        Self::for_namespace(Namespace::self_())
    }

    pub fn owner(&self) -> Option<Owner> {
        self.owner
    }

    pub fn namespace(&self) -> Option<&Namespace> {
        self.namespace.as_ref()
    }

    pub fn is_legacy_unscoped(&self) -> bool {
        self.owner.is_none() && self.namespace.is_none()
    }

    pub fn is_explicitly_scoped(&self) -> bool {
        self.owner.is_some() && self.namespace.is_some()
    }
}

impl<'de> serde::Deserialize<'de> for MemoryScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct MemoryScopeRepresentation {
            owner: Option<Owner>,
            namespace: Option<Namespace>,
        }

        let value = <MemoryScopeRepresentation as serde::Deserialize>::deserialize(deserializer)?;
        match (value.owner, value.namespace) {
            (None, None) => Ok(Self::legacy_unscoped()),
            (Some(owner), Some(namespace)) => {
                let scope = Self::for_namespace(namespace);
                if scope.owner() == Some(owner) {
                    Ok(scope)
                } else {
                    Err(serde::de::Error::custom(
                        "memory scope owner does not match namespace",
                    ))
                }
            }
            _ => Err(serde::de::Error::custom(
                "memory scope must be fully scoped or fully unscoped",
            )),
        }
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
