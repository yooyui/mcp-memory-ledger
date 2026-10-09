use crate::domain::{
    DomainError,
    types::{Mode, Namespace, Owner},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClaimReference(String);

impl ClaimReference {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.is_empty() || value.trim() != value || value.chars().any(char::is_whitespace) {
            return Err(DomainError::InvalidClaimReference);
        }

        let claim_id = value.strip_prefix("claim:").unwrap_or(&value);
        if claim_id.is_empty() {
            return Err(DomainError::InvalidClaimReference);
        }

        Ok(Self(claim_id.to_string()))
    }

    pub(crate) fn from_claim_id(claim_id: impl Into<String>) -> Self {
        Self(claim_id.into())
    }

    pub fn claim_id(&self) -> &str {
        &self.0
    }

    pub fn canonical(&self) -> String {
        format!("claim:{}", self.claim_id())
    }
}

impl serde::Serialize for ClaimReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.canonical())
    }
}

impl<'de> serde::Deserialize<'de> for ClaimReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::parse(value).map_err(|_| serde::de::Error::custom("invalid claim reference"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClaimDraft {
    owner: Owner,
    namespace: Namespace,
    subject: String,
    predicate: String,
    object: String,
    mode: Mode,
}

impl ClaimDraft {
    pub fn new(
        owner: Owner,
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
        mode: Mode,
    ) -> Self {
        Self::new_with_namespace(
            owner,
            Namespace::for_owner(owner),
            subject,
            predicate,
            object,
            mode,
        )
    }

    pub fn new_with_namespace(
        owner: Owner,
        namespace: Namespace,
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
        mode: Mode,
    ) -> Self {
        Self {
            owner,
            namespace,
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            mode,
        }
    }

    pub fn new_inferred(
        owner: Owner,
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
    ) -> Self {
        Self::new(owner, subject, predicate, object, Mode::Inferred)
    }

    pub fn with_namespace(mut self, namespace: Namespace) -> Self {
        self.namespace = namespace;
        self
    }

    pub fn validate(&self, evidence_count: usize) -> Result<(), DomainError> {
        if self.mode == Mode::Inferred && evidence_count == 0 {
            return Err(DomainError::InsufficientEvidence);
        }
        if !self.owner.is_accepted_for_new_writes() {
            return Err(DomainError::UnknownOwnerNotWritable);
        }

        self.validate_namespace_owner()?;

        Ok(())
    }

    pub fn validate_namespace_owner(&self) -> Result<(), DomainError> {
        if !self.namespace.matches_owner(self.owner) {
            return Err(DomainError::NamespaceOwnerMismatch);
        }

        Ok(())
    }

    pub fn owner(&self) -> Owner {
        self.owner
    }

    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn predicate(&self) -> &str {
        &self.predicate
    }

    pub fn object(&self) -> &str {
        &self.object
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }
}
