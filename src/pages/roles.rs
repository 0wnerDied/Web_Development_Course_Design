use crate::api;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn Roles() -> Element {
    let mut roles = use_signal(Vec::<api::Role>::new);
    let mut permissions = use_signal(Vec::<api::Permission>::new);
    let mut selected_role = use_signal(|| None::<api::Role>);
    let mut role_permissions = use_signal(Vec::<String>::new);
    let mut users = use_signal(Vec::<api::User>::new);

    let mut new_role_name = use_signal(String::new);
    let mut new_role_desc = use_signal(String::new);
    let mut selected_user_qq = use_signal(String::new);
    let mut selected_role_id = use_signal(|| 0i64);

    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let loading_visible = use_signal(|| false);

    {
        let loading = loading;
        let mut loading_visible = loading_visible;
        use_effect(move || {
            if *loading.read() {
                let loading = loading;
                let mut loading_visible = loading_visible;
                spawn(async move {
                    TimeoutFuture::new(180).await;
                    if *loading.read() {
                        loading_visible.set(true);
                    }
                });
            } else {
                loading_visible.set(false);
            }
        });
    }

    // 自动清除成功消息
    {
        let success = success;
        use_effect(move || {
            if success.read().is_some() {
                let mut success = success;
                spawn(async move {
                    TimeoutFuture::new(3000).await;
                    success.set(None);
                });
            }
        });
    }

    // 自动清除错误消息
    {
        let error = error;
        use_effect(move || {
            if error.read().is_some() {
                let mut error = error;
                spawn(async move {
                    TimeoutFuture::new(4000).await;
                    error.set(None);
                });
            }
        });
    }

    // 加载角色列表
    let load_roles = move || {
        spawn(async move {
            loading.set(true);
            match api::get_roles().await {
                Ok(role_list) => {
                    roles.set(role_list);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("加载角色失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 加载权限列表
    let load_permissions = move || {
        spawn(async move {
            match api::get_permissions().await {
                Ok(perm_list) => {
                    permissions.set(perm_list);
                }
                Err(e) => {
                    error.set(Some(format!("加载权限失败: {}", e)));
                }
            }
        });
    };

    // 加载用户列表
    let load_users = move || {
        spawn(async move {
            match api::get_users().await {
                Ok(user_list) => {
                    users.set(user_list);
                }
                Err(e) => {
                    error.set(Some(format!("加载用户失败: {}", e)));
                }
            }
        });
    };

    // 选择角色并加载其权限
    let mut select_role = move |role: api::Role| {
        let role_id = role.role_id;
        selected_role.set(Some(role));

        spawn(async move {
            match api::get_role_permissions(role_id).await {
                Ok(perms) => {
                    role_permissions.set(perms);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("加载角色权限失败: {}", e)));
                }
            }
        });
    };

    // 创建角色
    let mut create_role = move || {
        let name = new_role_name.read().clone();
        let desc = new_role_desc.read().clone();

        if name.is_empty() {
            error.set(Some("角色名称不能为空".to_string()));
            return;
        }

        spawn(async move {
            loading.set(true);
            match api::create_role(
                name.clone(),
                if desc.is_empty() { None } else { Some(desc) },
            )
            .await
            {
                Ok(msg) => {
                    success.set(Some(msg));
                    error.set(None);
                    new_role_name.set(String::new());
                    new_role_desc.set(String::new());
                    load_roles();
                }
                Err(e) => {
                    error.set(Some(format!("创建角色失败: {}", e)));
                    loading.set(false);
                }
            }
        });
    };

    // 删除角色
    let delete_role = move |role_id: i64| {
        spawn(async move {
            loading.set(true);
            match api::delete_role(role_id).await {
                Ok(msg) => {
                    success.set(Some(msg));
                    error.set(None);
                    selected_role.set(None);
                    load_roles();
                }
                Err(e) => {
                    error.set(Some(format!("删除角色失败: {}", e)));
                    loading.set(false);
                }
            }
        });
    };

    // 给角色分配权限
    let grant_permission = move |permission_name: String| {
        let Some(role) = selected_role.read().clone() else {
            return;
        };
        let role_id = role.role_id;

        spawn(async move {
            loading.set(true);
            match api::grant_permission_to_role(role_id, permission_name).await {
                Ok(msg) => {
                    success.set(Some(msg));
                    error.set(None);
                    // 重新加载角色权限
                    if let Ok(perms) = api::get_role_permissions(role_id).await {
                        role_permissions.set(perms)
                    }
                }
                Err(e) => {
                    error.set(Some(format!("分配权限失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 从角色移除权限
    let revoke_permission = move |permission_name: String| {
        let Some(role) = selected_role.read().clone() else {
            return;
        };
        let role_id = role.role_id;

        spawn(async move {
            loading.set(true);
            match api::revoke_permission_from_role(role_id, permission_name).await {
                Ok(msg) => {
                    success.set(Some(msg));
                    error.set(None);
                    // 重新加载角色权限
                    if let Ok(perms) = api::get_role_permissions(role_id).await {
                        role_permissions.set(perms)
                    }
                }
                Err(e) => {
                    error.set(Some(format!("移除权限失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 给用户分配角色
    let mut assign_role = move || {
        let user_qq = selected_user_qq.read().clone();
        let role_id = *selected_role_id.read();

        if user_qq.is_empty() || role_id == 0 {
            error.set(Some("请选择用户和角色".to_string()));
            return;
        }

        spawn(async move {
            loading.set(true);
            match api::assign_role_to_user(user_qq.clone(), role_id).await {
                Ok(msg) => {
                    success.set(Some(format!("{} - 已为用户 {} 分配角色", msg, user_qq)));
                    error.set(None);
                    selected_user_qq.set(String::new());
                    selected_role_id.set(0);
                    // 刷新用户列表以显示更新后的角色
                    load_users();
                }
                Err(e) => {
                    error.set(Some(format!("分配角色失败: {}", e)));
                    success.set(None);
                }
            }
            loading.set(false);
        });
    };

    // 初始加载
    use_effect(move || {
        load_roles();
        load_permissions();
        load_users();
    });

    rsx! {
        div { class: "page-container",
            h1 { "角色管理" }

            div { class: "toolbar",
                button {
                    class: "btn-secondary",
                    onclick: move |_| {
                        load_roles();
                        load_permissions();
                        load_users();
                    },
                    disabled: *loading.read(),
                    "刷新"
                }
            }

            if *loading_visible.read() {
                div { class: "loading-message", "加载中..." }
            }

            // Toast 通知容器
            div { class: "toast-container",
                if let Some(err) = error.read().as_ref() {
                    div { class: "toast toast-error", "{err}" }
                }

                if let Some(succ) = success.read().as_ref() {
                    div { class: "toast toast-success", "{succ}" }
                }
            }

            div { class: "permission-layout",
                // 左侧：角色列表和创建表单
                div { class: "user-list-panel",
                    h3 {
                        style: "display: flex; align-items: center; gap: 0.5rem;",
                        "角色列表"
                    }

                    div { class: "user-list",
                        for role in roles.read().iter() {
                            div {
                                key: "{role.role_id}",
                                class: if let Some(sel) = selected_role.read().as_ref() {
                                    if sel.role_id == role.role_id { "user-item active" } else { "user-item" }
                                } else { "user-item" },
                                onclick: {
                                    let role = role.clone();
                                    move |_| select_role(role.clone())
                                },
                                div { strong { "{role.name}" } }
                                if let Some(desc) = &role.description {
                                    div { style: "font-size: 12px; color: #666;", "{desc}" }
                                }
                            }
                        }
                    }

                    h3 {
                        style: "margin-top: 2rem; display: flex; align-items: center; gap: 0.5rem;",
                        "创建新角色"
                    }
                    div { class: "form-group",
                        label { "角色名称" }
                        input {
                            r#type: "text",
                            placeholder: "输入角色名称",
                            value: "{new_role_name}",
                            oninput: move |evt| new_role_name.set(evt.value().clone()),
                        }
                    }
                    div { class: "form-group",
                        label { "角色描述" }
                        input {
                            r#type: "text",
                            placeholder: "输入角色描述（可选）",
                            value: "{new_role_desc}",
                            oninput: move |evt| new_role_desc.set(evt.value().clone()),
                        }
                    }
                    button {
                        class: "btn-primary",
                        onclick: move |_| create_role(),
                        disabled: *loading.read(),
                        "创建角色"
                    }
                }

                // 右侧：角色权限管理
                div { class: "permission-panel",
                    if let Some(role) = selected_role.read().as_ref() {
                        h2 { "{role.name}" }
                        if let Some(desc) = &role.description {
                            p { style: "color: #666; margin-bottom: 1rem;", "{desc}" }
                        }

                        button {
                            class: "btn-danger btn-small",
                            onclick: {
                                let role_id = role.role_id;
                                move |_| delete_role(role_id)
                            },
                            disabled: *loading.read(),
                            style: "margin-bottom: 1.5rem; display: inline-flex; align-items: center; gap: 0.375rem;",
                            "删除此角色"
                        }

                        h3 {
                            style: "display: flex; align-items: center; gap: 0.5rem;",
                            "权限配置"
                        }
                        div { class: "permission-grid",
                            for perm in permissions.read().iter() {
                                {
                                    let has_perm = role_permissions.read().contains(&perm.name);
                                    let perm_name = perm.name.clone();

                                    rsx! {
                                        div {
                                            key: "{perm.name}",
                                            class: "permission-item",
                                            span { "{perm.name}" }
                                            if has_perm {
                                                button {
                                                    class: "btn-small btn-danger",
                                                    onclick: move |_| revoke_permission(perm_name.clone()),
                                                    disabled: *loading.read(),
                                                    "移除"
                                                }
                                            } else {
                                                button {
                                                    class: "btn-small btn-success",
                                                    onclick: move |_| grant_permission(perm_name.clone()),
                                                    disabled: *loading.read(),
                                                    "添加"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "current-permissions",
                            h3 {
                                style: "display: flex; align-items: center; gap: 0.5rem;",
                                span { style: "font-size: 14px;", "✅" }
                                "已分配的权限"
                            }
                            if role_permissions.read().is_empty() {
                                p { "该角色暂无权限" }
                            } else {
                                ul {
                                    for perm in role_permissions.read().iter() {
                                        li { key: "{perm}", "{perm}" }
                                    }
                                }
                            }
                        }
                    } else {
                        div {
                            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 4rem 2rem; color: #999;",
                            div { style: "font-size: 32px; font-weight: 500;", "请从左侧选择一个角色" }
                            div { style: "font-size: 24px; margin-top: 0.5rem; color: #bbb;", "选择角色后即可管理其权限" }
                        }
                    }
                }
            }

            // 给用户分配角色
            div { class: "info-section",
                h2 { "为用户分配角色" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr auto; gap: 1.5rem; align-items: end;",
                    div { style: "margin-bottom: 0;",
                        label {
                            style: "display: block; margin-bottom: 0.625rem; font-weight: 600; color: #333; font-size: 14px;",
                            "选择用户"
                        }
                        select {
                            style: "width: 100%; padding: 0.875rem 1.125rem; border: 2px solid #e0e0e0; border-radius: 10px; font-size: 14px; background: #f8f9fa; color: #333;",
                            value: "{selected_user_qq}",
                            onchange: move |evt| selected_user_qq.set(evt.value().clone()),
                            option { value: "", "-- 请选择用户 --" }
                            for user in users.read().iter() {
                                option {
                                    key: "{user.qq}",
                                    value: "{user.qq}",
                                    "{user.nickname} ({user.qq})"
                                }
                            }
                        }
                    }
                    div { style: "margin-bottom: 0;",
                        label {
                            style: "display: block; margin-bottom: 0.625rem; font-weight: 600; color: #333; font-size: 14px;",
                            "选择角色"
                        }
                        select {
                            style: "width: 100%; padding: 0.875rem 1.125rem; border: 2px solid #e0e0e0; border-radius: 10px; font-size: 14px; background: #f8f9fa; color: #333;",
                            value: "{selected_role_id}",
                            onchange: move |evt| {
                                if let Ok(id) = evt.value().parse::<i64>() {
                                    selected_role_id.set(id);
                                }
                            },
                            option { value: "0", "-- 请选择角色 --" }
                            for role in roles.read().iter() {
                                option {
                                    key: "{role.role_id}",
                                    value: "{role.role_id}",
                                    "{role.name}"
                                }
                            }
                        }
                    }
                    button {
                        class: "btn-primary",
                        style: "height: 46px; padding: 0 2rem; white-space: nowrap;",
                        onclick: move |_| assign_role(),
                        disabled: *loading.read(),
                        "分配角色"
                    }
                }
            }
        }
    }
}
