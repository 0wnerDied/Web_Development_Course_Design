use dioxus::prelude::*;
use crate::api;

#[component]
pub fn Permissions() -> Element {
    let mut users = use_signal(Vec::<api::User>::new);
    let mut permissions = use_signal(Vec::<api::Permission>::new);
    let mut selected_user = use_signal(|| None::<String>);
    let mut user_permissions = use_signal(Vec::<api::Permission>::new);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    
    let load_data = move || {
        spawn(async move {
            loading.set(true);
            match api::get_users().await {
                Ok(user_list) => { users.set(user_list); }
                Err(e) => { error.set(Some(format!("加载用户失败: {}", e))); }
            }
            match api::get_permissions().await {
                Ok(perm_list) => { permissions.set(perm_list); }
                Err(e) => { error.set(Some(format!("加载权限失败: {}", e))); }
            }
            loading.set(false);
        });
    };
    
    let load_user_permissions = move |qq: String| {
        spawn(async move {
            loading.set(true);
            match api::get_user_permissions(&qq).await {
                Ok(perms) => {
                    user_permissions.set(perms);
                    selected_user.set(Some(qq));
                    error.set(None);
                }
                Err(e) => { error.set(Some(format!("加载用户权限失败: {}", e))); }
            }
            loading.set(false);
        });
    };
    
    let grant_permission = move |perm_id: i64| {
        let user_qq_opt = selected_user.read().clone();
        let Some(user_qq) = user_qq_opt else { return; };
        spawn(async move {
            loading.set(true);
            match api::grant_permission(user_qq.clone(), perm_id).await {
                Ok(_) => {
                    success.set(Some("已授予权限".to_string()));
                    error.set(None);
                    load_user_permissions(user_qq);
                }
                Err(e) => { error.set(Some(format!("授予权限失败: {}", e))); }
            }
            loading.set(false);
        });
    };
    
    let revoke_permission = move |perm_id: i64| {
        let user_qq_opt = selected_user.read().clone();
        let Some(user_qq) = user_qq_opt else { return; };
        spawn(async move {
            loading.set(true);
            match api::revoke_permission(user_qq.clone(), perm_id).await {
                Ok(_) => {
                    success.set(Some("已撤销权限".to_string()));
                    error.set(None);
                    load_user_permissions(user_qq);
                }
                Err(e) => { error.set(Some(format!("撤销权限失败: {}", e))); }
            }
            loading.set(false);
        });
    };
    
    use_effect(move || { load_data(); });

    let user_has_permission = |perm_id: i64| -> bool {
        user_permissions.read().iter().any(|p| p.id == perm_id)
    };

    rsx! {
        div { class: "page-container",
            h1 { "权限管理" }
            if *loading.read() { div { class: "loading-message", "处理中..." } }
            if let Some(err) = error.read().as_ref() { div { class: "error-message", "{err}" } }
            if let Some(succ) = success.read().as_ref() { div { class: "success-message", "{succ}" } }
            div { class: "permission-layout",
                div { class: "user-list-panel",
                    h2 { "选择用户" }
                    button { class: "btn-secondary", onclick: move |_| load_data(), disabled: *loading.read(), "刷新" }
                    div { class: "user-list",
                        for user in users.read().iter() {
                            div {
                                key: "{user.qq}",
                                class: if selected_user.read().as_ref() == Some(&user.qq) { "user-item active" } else { "user-item" },
                                onclick: { let qq = user.qq.clone(); move |_| load_user_permissions(qq.clone()) },
                                "{user.nickname} ({user.qq})"
                            }
                        }
                    }
                }
                div { class: "permission-panel",
                    if let Some(user_qq) = selected_user.read().as_ref() {
                        h2 { "用户权限配置 - {user_qq}" }
                        div { class: "permission-grid",
                            for perm in permissions.read().iter() {
                                div { class: "permission-item",
                                    span { class: "perm-name", "{perm.name}" }
                                    if let Some(desc) = &perm.description { span { class: "perm-desc", " - {desc}" } }
                                    if user_has_permission(perm.id) {
                                        button { class: "btn-small btn-danger", onclick: { let id = perm.id; move |_| revoke_permission(id) }, disabled: *loading.read(), "撤销" }
                                    } else {
                                        button { class: "btn-small btn-success", onclick: { let id = perm.id; move |_| grant_permission(id) }, disabled: *loading.read(), "授予" }
                                    }
                                }
                            }
                        }
                        div { class: "current-permissions",
                            h3 { "当前权限列表" }
                            if user_permissions.read().is_empty() {
                                p { "该用户暂无任何权限" }
                            } else {
                                ul {
                                    for perm in user_permissions.read().iter() {
                                        li { key: "{perm.id}", "{perm.name}" if let Some(desc) = &perm.description { span { class: "perm-desc", " - {desc}" } } }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "empty-state", "请从左侧选择一个用户" }
                    }
                }
            }
        }
    }
}