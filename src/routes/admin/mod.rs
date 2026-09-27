use super::State;
use utoipa_axum::router::OpenApiRouter;

mod curseforge;
mod settings;

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest("/settings", settings::router(state))
        .nest("/curseforge", curseforge::router(state))
        .with_state(state.clone())
}
