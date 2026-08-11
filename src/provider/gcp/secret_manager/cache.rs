//! Cache keys for the Secret Manager service, and the invalidation rules that
//! keep them honest after a mutation.

use crate::cache::{Cache, CacheKey};
use crate::provider::gcp::secret_manager::payload::SecretPayload;
use crate::provider::gcp::secret_manager::secrets::Secret;
use crate::provider::gcp::secret_manager::versions::SecretVersion;

/// The secrets of the current project.
#[derive(Debug, Hash, PartialEq, Eq)]
pub(super) struct SecretsKey;

impl CacheKey for SecretsKey {
    type Value = Vec<Secret>;
}

/// The versions of one secret, by secret name.
#[derive(Debug, Hash, PartialEq, Eq)]
pub(super) struct VersionsKey(pub String);

impl CacheKey for VersionsKey {
    type Value = Vec<SecretVersion>;
}

/// The payload of one secret version, or of its latest version.
#[derive(Debug, Hash, PartialEq, Eq)]
pub(super) struct PayloadKey {
    pub secret: String,
    /// `None` addresses the `latest` alias rather than a fixed version.
    pub version: Option<String>,
}

impl CacheKey for PayloadKey {
    type Value = SecretPayload;
}

impl PayloadKey {
    pub fn new(secret: &Secret, version: Option<&SecretVersion>) -> Self {
        Self {
            secret: secret.name.clone(),
            version: version.map(|v| v.version_id.clone()),
        }
    }
}

/// Drop everything cached about the versions of `secret_name`.
///
/// Any mutation of a secret's versions can invalidate more than the version
/// list: adding one moves the `latest` alias onto new data, and
/// disabling/destroying one changes whether a payload can be read at all. So
/// every cached payload of the secret goes too, not just the list.
pub(super) fn invalidate_secret(cache: &mut Cache, secret_name: &str) {
    cache.invalidate(&VersionsKey(secret_name.to_string()));
    cache.invalidate_where::<PayloadKey>(|key| key.secret == secret_name);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload_key(secret: &str, version: Option<&str>) -> PayloadKey {
        PayloadKey {
            secret: secret.to_string(),
            version: version.map(str::to_string),
        }
    }

    fn payload(data: &str) -> SecretPayload {
        SecretPayload {
            data: data.to_string(),
            is_binary: false,
        }
    }

    #[test]
    fn invalidate_secret_drops_the_latest_payload() {
        let mut cache = Cache::new();
        cache.insert(payload_key("db", None), payload("old"));

        invalidate_secret(&mut cache, "db");

        assert_eq!(cache.get(&payload_key("db", None)), None);
    }

    #[test]
    fn invalidate_secret_drops_pinned_version_payloads() {
        let mut cache = Cache::new();
        cache.insert(payload_key("db", Some("1")), payload("v1"));

        invalidate_secret(&mut cache, "db");

        assert_eq!(cache.get(&payload_key("db", Some("1"))), None);
    }

    #[test]
    fn invalidate_secret_drops_the_version_list() {
        let mut cache = Cache::new();
        cache.insert(VersionsKey("db".to_string()), Vec::new());

        invalidate_secret(&mut cache, "db");

        assert_eq!(cache.get(&VersionsKey("db".to_string())), None);
    }

    #[test]
    fn invalidate_secret_leaves_other_secrets_alone() {
        let mut cache = Cache::new();
        cache.insert(payload_key("api", None), payload("api-latest"));
        cache.insert(VersionsKey("api".to_string()), Vec::new());

        invalidate_secret(&mut cache, "db");

        assert_eq!(
            cache.get(&payload_key("api", None)),
            Some(&payload("api-latest"))
        );
        assert_eq!(
            cache.get(&VersionsKey("api".to_string())),
            Some(&Vec::new())
        );
    }
}
