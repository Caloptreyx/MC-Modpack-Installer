use super::State;
use crate::providers::{Client, ModpackDetails, Provider, user_error};
use axum::http::StatusCode;
use utoipa_axum::router::OpenApiRouter;

mod _project_;

/// Resolves the platform client and project named by the `{provider}/{project}` path segments.
async fn resolve(
    state: &State,
    provider: &str,
    project: &str,
) -> Result<(Client, ModpackDetails), anyhow::Error> {
    let provider = Provider::parse(provider)
        .ok_or_else(|| user_error("unknown modpack platform", StatusCode::NOT_FOUND))?;
    let settings = crate::settings::load(state).await?;
    let client = Client::new(&settings, provider)?;
    let details = client.project(state, project).await?;

    Ok((client, details))
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest("/{provider}/{project}", _project_::router(state))
        .with_state(state.clone())
}
