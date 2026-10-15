use agent_llm_mm::domain::{
    claim::ClaimDraft,
    types::{MemoryScope, Mode, Namespace, Owner},
};

#[test]
fn inferred_claim_requires_external_evidence() {
    let draft = ClaimDraft::new_inferred(Owner::Self_, "self.role", "is", "architect");
    assert!(draft.validate(0).is_err());
}

#[test]
fn identity_core_updates_are_not_allowed_from_any_ingest_mode() {
    let ingest_modes = [Mode::Observed, Mode::Said, Mode::Acted, Mode::Inferred];

    for mode in ingest_modes {
        let result = agent_llm_mm::domain::identity_core::allow_direct_ingest_update(mode);
        assert!(
            !result,
            "identity_core direct updates should stay blocked for ingest mode {:?}",
            mode
        );
    }
}

#[test]
fn claim_defaults_namespace_from_owner_scope() {
    let self_claim = ClaimDraft::new(Owner::Self_, "self.role", "is", "architect", Mode::Observed);
    let user_claim = ClaimDraft::new(
        Owner::User,
        "user.preference",
        "likes",
        "concise",
        Mode::Observed,
    );
    let world_claim = ClaimDraft::new(
        Owner::World,
        "project.memory",
        "needs",
        "structure",
        Mode::Observed,
    );

    assert_eq!(self_claim.namespace().as_str(), "self");
    assert_eq!(user_claim.namespace().as_str(), "user/default");
    assert_eq!(world_claim.namespace().as_str(), "world");
}

#[test]
fn explicit_namespace_must_match_owner_scope() {
    let draft = ClaimDraft::new_with_namespace(
        Owner::Self_,
        Namespace::for_user("default"),
        "self.role",
        "is",
        "architect",
        Mode::Observed,
    );

    assert!(draft.validate(1).is_err());
}

#[test]
fn unknown_owner_is_not_accepted_for_new_writes() {
    assert!(!Owner::Unknown.is_accepted_for_new_writes());
    for owner in [Owner::Self_, Owner::User, Owner::World] {
        assert!(owner.is_accepted_for_new_writes());
    }

    let draft = ClaimDraft::new_with_namespace(
        Owner::Unknown,
        Namespace::world(),
        "world.fact",
        "is",
        "legacy",
        Mode::Observed,
    );
    assert_eq!(
        draft.validate(1),
        Err(agent_llm_mm::domain::DomainError::UnknownOwnerNotWritable)
    );
}

#[test]
fn canonical_namespace_pairs_derive_one_write_owner() {
    for (namespace, owner) in [
        (Namespace::self_(), Owner::Self_),
        (Namespace::world(), Owner::World),
        (Namespace::for_user("alice"), Owner::User),
        (Namespace::for_project("demo"), Owner::World),
    ] {
        assert_eq!(namespace.derived_owner(), owner);
        assert_eq!(
            MemoryScope::for_namespace(namespace.clone()).owner(),
            Some(owner)
        );
        assert_eq!(
            MemoryScope::for_namespace(namespace.clone()).namespace(),
            Some(&namespace)
        );
    }
}

#[test]
fn namespace_and_memory_scope_deserialization_preserve_scope_invariants() {
    assert!(serde_json::from_value::<Namespace>(serde_json::json!("tenant/invalid")).is_err());

    let legacy = serde_json::from_value::<MemoryScope>(serde_json::json!({
        "owner": null,
        "namespace": null
    }))
    .expect("fully empty compatibility scope should remain valid");
    assert!(legacy.is_legacy_unscoped());

    let scoped = serde_json::from_value::<MemoryScope>(serde_json::json!({
        "owner": "World",
        "namespace": "project/agent-llm-mm"
    }))
    .expect("matching owner and namespace should deserialize");
    assert!(scoped.is_explicitly_scoped());

    for invalid in [
        serde_json::json!({"owner": "World", "namespace": null}),
        serde_json::json!({"owner": null, "namespace": "world"}),
        serde_json::json!({"owner": "User", "namespace": "project/agent-llm-mm"}),
        serde_json::json!({"owner": "World", "namespace": "tenant/invalid"}),
    ] {
        assert!(serde_json::from_value::<MemoryScope>(invalid).is_err());
    }
}
