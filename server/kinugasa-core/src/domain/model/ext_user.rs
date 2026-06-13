use crate::domain::model::id;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalUserId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedExternalUser {
    pub id: ExternalUserId,
    pub display_name: String,
    pub groups: Vec<id::ExternalGroupKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalUserCredential {
    BearerToken(String),
}

#[async_trait::async_trait]
pub trait ExternalUserAuthenticator: Clone + Send + Sync + 'static {
    async fn authenticate(
        &self,
        credential: ExternalUserCredential,
    ) -> anyhow::Result<Option<AuthenticatedExternalUser>>;
}
