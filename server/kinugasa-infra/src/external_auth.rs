use async_trait::async_trait;
use kinugasa_core::domain::model::{
    ext_user::{AuthenticatedExternalUser, ExternalUserAuthenticator, ExternalUserCredential},
    id,
};

#[derive(Debug, Clone)]
pub struct MockExternalUserAuthenticator {
    expected_bearer_token: String,
    authenticated_user: AuthenticatedExternalUser,
}

impl MockExternalUserAuthenticator {
    pub fn new(
        expected_bearer_token: String,
        authenticated_user: AuthenticatedExternalUser,
    ) -> Self {
        Self {
            expected_bearer_token,
            authenticated_user,
        }
    }

    pub fn dev(expected_bearer_token: String) -> Self {
        Self::new(
            expected_bearer_token,
            AuthenticatedExternalUser {
                id: kinugasa_core::domain::model::ext_user::ExternalUserId("mock-user".to_owned()),
                display_name: "Mock User".to_owned(),
                groups: vec![id::ExternalGroupKey("mock-group".to_owned())],
            },
        )
    }
}

#[async_trait]
impl ExternalUserAuthenticator for MockExternalUserAuthenticator {
    async fn authenticate(
        &self,
        credential: ExternalUserCredential,
    ) -> anyhow::Result<Option<AuthenticatedExternalUser>> {
        match credential {
            ExternalUserCredential::BearerToken(token) if token == self.expected_bearer_token => {
                Ok(Some(self.authenticated_user.clone()))
            }
            ExternalUserCredential::BearerToken(_) => Ok(None),
        }
    }
}
