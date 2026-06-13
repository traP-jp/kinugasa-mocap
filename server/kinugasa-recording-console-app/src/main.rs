use std::{env, net::SocketAddr};

use axum::{
    Router, middleware,
    routing::{get, post},
};
use kinugasa_core::usecase::{
    auth::middleware as auth_middleware,
    health::handlers as health_handlers,
    mocap_team::{handlers as mocap_team_handlers, state::MocapTeamApiState},
};
use kinugasa_infra::{
    external_auth::MockExternalUserAuthenticator,
    mysql::{MySqlInfra, MySqlMocapTeamRepository, MySqlUnitOfWorkProvider},
};

const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_AUTH_MOCK_BEARER_TOKEN: &str = "dev-token";

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let infra = MySqlInfra::connect(&database_url)
        .await
        .expect("connect to recording console database");
    let mocap_team_api_state =
        MocapTeamApiState::new(infra.mocap_team_repository(), infra.unit_of_work_provider());
    let authenticator = MockExternalUserAuthenticator::dev(
        env::var("AUTH_MOCK_BEARER_TOKEN")
            .unwrap_or_else(|_| DEFAULT_AUTH_MOCK_BEARER_TOKEN.to_owned()),
    );

    let app = Router::new()
        .route("/healthz", get(health_handlers::get_health))
        .route(
            "/mocap-teams",
            get(mocap_team_handlers::list_mocap_teams::<
                MySqlMocapTeamRepository,
                MySqlUnitOfWorkProvider,
            >),
        )
        .route(
            "/mocap-teams/{externalUsergroupKey}",
            post(
                mocap_team_handlers::create_mocap_team::<
                    MySqlMocapTeamRepository,
                    MySqlUnitOfWorkProvider,
                >,
            ),
        )
        .route(
            "/mocap-teams/by-external-usergroup-key/{externalUsergroupKey}",
            get(
                mocap_team_handlers::get_mocap_team_by_external_usergroup_key::<
                    MySqlMocapTeamRepository,
                    MySqlUnitOfWorkProvider,
                >,
            ),
        )
        .route(
            "/mocap-teams/by-id/{teamId}",
            get(mocap_team_handlers::get_mocap_team::<
                MySqlMocapTeamRepository,
                MySqlUnitOfWorkProvider,
            >),
        )
        .with_state(mocap_team_api_state)
        .layer(middleware::from_fn_with_state(
            authenticator,
            auth_middleware::require_external_user::<MockExternalUserAuthenticator>,
        ));
    let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| DEFAULT_LISTEN_ADDR.to_owned());
    let listener = tokio::net::TcpListener::bind(parse_listen_addr(&listen_addr))
        .await
        .expect("bind recording console HTTP listener");

    axum::serve(listener, app)
        .await
        .expect("serve recording console HTTP API");
}

fn parse_listen_addr(value: &str) -> SocketAddr {
    value
        .parse()
        .expect("LISTEN_ADDR must be a socket address like 0.0.0.0:8080")
}
