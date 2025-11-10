use dioxus::prelude::*;

use crate::models::SessionUser;

#[derive(Clone, Copy)]
pub struct AppContext {
    pub current_user: Signal<Option<SessionUser>>,
    pub is_loading: Signal<bool>,
}

pub fn use_app_context() -> AppContext {
    use_context::<AppContext>()
}

pub fn use_current_user() -> Signal<Option<SessionUser>> {
    use_app_context().current_user
}
