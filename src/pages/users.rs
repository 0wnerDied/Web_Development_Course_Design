use crate::api;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn Users() -> Element {
    let mut users = use_signal(Vec::<api::User>::new);
    let mut search_keyword = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
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

    // 加载所有用户
    let load_users = move || {
        spawn(async move {
            loading.set(true);
            match api::get_users().await {
                Ok(user_list) => {
                    users.set(user_list);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("加载失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 搜索用户(前端过滤)
    let on_search = move || {
        let keyword = search_keyword.read().clone().to_lowercase();
        spawn(async move {
            loading.set(true);
            match api::get_users().await {
                Ok(user_list) => {
                    let filtered: Vec<_> = if keyword.is_empty() {
                        user_list
                    } else {
                        user_list
                            .into_iter()
                            .filter(|u| {
                                u.qq.to_lowercase().contains(&keyword)
                                    || u.nickname.to_lowercase().contains(&keyword)
                            })
                            .collect()
                    };
                    users.set(filtered);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("搜索失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 删除用户
    let delete_user = move |qq: String| {
        spawn(async move {
            loading.set(true);
            match api::delete_user(&qq).await {
                Ok(_) => {
                    error.set(None);
                    load_users(); // 重新加载列表
                }
                Err(e) => {
                    error.set(Some(format!("删除失败: {}", e)));
                    loading.set(false);
                }
            }
        });
    };

    // 初始加载
    use_effect(move || {
        load_users();
    });

    rsx! {
        div { class: "page-container",
            h1 { "用户管理" }

            div { class: "toolbar",
                div { class: "search-box",
                    input {
                        r#type: "text",
                        placeholder: "搜索QQ号或昵称...",
                        value: "{search_keyword}",
                        oninput: move |evt| search_keyword.set(evt.value().clone()),
                        disabled: *loading.read()
                    }
                    button {
                        class: "btn-primary",
                        onclick: move |_| on_search(),
                        disabled: *loading.read(),
                        "搜索"
                    }
                    button {
                        class: "btn-secondary",
                        onclick: move |_| load_users(),
                        disabled: *loading.read(),
                        "刷新"
                    }
                }
            }

            if *loading_visible.read() {
                div { class: "loading-message", "加载中..." }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "error-message", "{err}" }
            }

            div { class: "table-container",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "QQ号" }
                            th { "昵称" }
                            th { "角色" }
                            th { "生日" }
                            th { "操作" }
                        }
                    }
                    tbody {
                        for user in users.read().iter() {
                            tr {
                                key: "{user.qq}",
                                td { "{user.qq}" }
                                td { "{user.nickname}" }
                                td {
                                    if let Some(role_name) = &user.role_name {
                                        span {
                                            style: "padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 4px; font-size: 12px;",
                                            "{role_name}"
                                        }
                                    } else {
                                        span { style: "color: #999;", "无角色" }
                                    }
                                }
                                td {
                                    if let Some(birthday) = &user.birthday {
                                        "{birthday}"
                                    } else {
                                        "-"
                                    }
                                }
                                td {
                                    Link {
                                        to: crate::Route::Roles {},
                                        button {
                                            class: "btn-small btn-info",
                                            title: "跳转到角色管理页面设置用户角色",
                                            "设置角色"
                                        }
                                    }
                                    button {
                                        class: "btn-small btn-danger",
                                        onclick: {
                                            let qq = user.qq.clone();
                                            move |_| delete_user(qq.clone())
                                        },
                                        disabled: *loading.read(),
                                        "删除用户"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "stats",
                "共 {users.read().len()} 个用户"
            }
        }
    }
}
