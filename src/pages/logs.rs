use crate::api;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

const PAGE_SIZE: i32 = 20;

#[component]
pub fn Logs() -> Element {
    let mut logs = use_signal(Vec::<api::RequestLog>::new);
    let mut total = use_signal(|| 0i64);
    let mut page = use_signal(|| 0);
    let mut error = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let loading_visible = use_signal(|| false);
    let mut filter_user = use_signal(String::new);
    let mut input_value = use_signal(String::new);

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

    let load_logs = move || {
        let current_page = *page.read();
        let filter_value = filter_user.read().clone();

        spawn(async move {
            loading.set(true);
            let offset = (current_page * PAGE_SIZE).max(0);
            let trimmed = filter_value.trim().to_string();
            let user_qq = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            };

            match api::get_request_logs(Some(PAGE_SIZE), Some(offset), user_qq).await {
                Ok(response) => {
                    logs.set(response.logs);
                    total.set(response.total);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("加载日志失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    let prev_page = move |_| {
        let current = *page.read();
        if current > 0 {
            page.set(current - 1);
            load_logs();
        }
    };

    let next_page = move |_| {
        let current = *page.read();
        let total_logs = *total.read();
        let total_pages = calc_total_pages(total_logs);
        if total_pages == 0 {
            return;
        }
        if (current as i64) < (total_pages as i64 - 1) {
            page.set(current + 1);
            load_logs();
        }
    };

    let on_input_change = move |evt: Event<FormData>| {
        input_value.set(evt.value().clone());
    };

    let apply_filter = move |evt: Event<FormData>| {
        evt.prevent_default();
        let value = input_value.read().clone();
        let trimmed = value.trim().to_string();
        filter_user.set(trimmed);
        page.set(0);
        load_logs();
    };

    let reset_filter = move |_| {
        filter_user.set(String::new());
        input_value.set(String::new());
        page.set(0);
        load_logs();
    };

    use_effect(move || {
        load_logs();
    });

    let logs_snapshot = logs.read().clone();
    let total_count = *total.read();
    let current_page_value = *page.read();
    let is_loading = *loading.read();
    let show_loading_indicator = *loading_visible.read();
    let error_message = error.read().clone();

    let total_pages = calc_total_pages(total_count);
    let displayed_page = if total_pages > 0 {
        current_page_value + 1
    } else {
        0
    };
    let has_prev_page = current_page_value > 0;
    let has_next_page = total_pages > 0 && (current_page_value as i64) < (total_pages as i64 - 1);
    let showing_range = if logs_snapshot.is_empty() || total_count == 0 {
        None
    } else {
        let start = (current_page_value as i64) * PAGE_SIZE as i64 + 1;
        let end = start + logs_snapshot.len() as i64 - 1;
        Some((start, end))
    };

    let stats_text = if let Some((start, end)) = showing_range {
        format!(
            "显示第 {} - {} 条，共 {} 条记录（第 {} / {} 页）",
            start,
            end,
            total_count,
            displayed_page,
            total_pages.max(1)
        )
    } else {
        "暂无日志数据".to_string()
    };

    let pagination_label = if total_pages > 0 {
        format!("第 {} / {} 页", displayed_page, total_pages)
    } else {
        "第 0 / 0 页".to_string()
    };

    rsx! {
        div { class: "page-container",
            h1 { "系统日志" }

            div { class: "toolbar",
                form {
                    class: "search-box",
                    onsubmit: apply_filter,
                    input {
                        r#type: "text",
                        name: "user_qq",
                        id: "filter-input",
                        placeholder: "按用户QQ筛选（留空为全部）",
                        disabled: is_loading,
                        oninput: on_input_change,
                    }
                    button {
                        r#type: "submit",
                        class: "btn-primary",
                        disabled: is_loading,
                        "应用筛选"
                    }
                    button {
                        r#type: "button",
                        class: "btn-secondary",
                        onclick: reset_filter,
                        disabled: is_loading,
                        "重置"
                    }
                }
                button {
                    class: "btn-secondary",
                    onclick: move |_| load_logs(),
                    disabled: is_loading,
                    "刷新"
                }
            }

            if show_loading_indicator {
                div { class: "loading-message", "加载中，请稍候..." }
            }

            if let Some(err) = error_message.as_ref() {
                div { class: "error-message", "{err}" }
            }

            if logs_snapshot.is_empty() {
                div { class: "empty-state", "暂无日志记录" }
            } else {
                div { class: "table-container",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "ID" }
                                th { "时间" }
                                th { "方法" }
                                th { "路径" }
                                th { "用户QQ" }
                                th { "状态码" }
                                th { "请求体" }
                            }
                        }
                        tbody {
                            for log in logs_snapshot.iter() {
                                tr { key: "{log.id}",
                                    td { "{log.id}" }
                                    td { class: "mono-cell", "{log.timestamp}" }
                                    td {
                                        span {
                                            class: method_badge_class(&log.method),
                                            "{log.method}"
                                        }
                                    }
                                    td { class: "mono-cell log-url-cell", "{log.path}" }
                                    td {
                                        if let Some(qq) = &log.user_qq {
                                            "{qq}"
                                        } else {
                                            "-"
                                        }
                                    }
                                    td {
                                        span {
                                            class: status_badge_class(log.status),
                                            "{log.status}"
                                        }
                                    }
                                    td {
                                        div {
                                            class: "log-body-cell mono-cell",
                                            title: "{body_preview_title(&log.body)}",
                                            "{body_preview_text(&log.body)}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "stats", "{stats_text}" }

            div { class: "pagination-bar",
                span { "{pagination_label}" }
                div { class: "pagination-actions",
                    button {
                        class: "btn-secondary",
                        onclick: prev_page,
                        disabled: !has_prev_page || is_loading,
                        "上一页"
                    }
                    button {
                        class: "btn-secondary",
                        onclick: next_page,
                        disabled: !has_next_page || is_loading,
                        "下一页"
                    }
                }
            }
        }
    }
}

fn calc_total_pages(total: i64) -> i32 {
    if total <= 0 {
        0
    } else {
        (((total - 1) / PAGE_SIZE as i64) + 1) as i32
    }
}

fn method_badge_class(method: &str) -> &'static str {
    match method {
        "GET" => "badge badge-success",
        "POST" => "badge badge-info",
        "PUT" | "PATCH" => "badge badge-warning",
        "DELETE" => "badge badge-danger",
        _ => "badge",
    }
}

fn status_badge_class(status: i32) -> &'static str {
    match status {
        200..=299 => "badge badge-success",
        400..=499 => "badge badge-warning",
        500..=599 => "badge badge-danger",
        _ => "badge",
    }
}

fn body_preview_text(body: &Option<String>) -> String {
    body.as_ref()
        .map(|text| {
            let normalized = normalize_whitespace(text);
            if normalized.is_empty() {
                "-".to_string()
            } else if normalized.chars().nth(80).is_some() {
                let preview: String = normalized.chars().take(80).collect();
                format!("{}...", preview)
            } else {
                normalized
            }
        })
        .unwrap_or_else(|| "-".to_string())
}

fn body_preview_title(body: &Option<String>) -> String {
    body.as_ref()
        .map(|text| {
            let normalized = normalize_whitespace(text);
            if normalized.is_empty() {
                "-".to_string()
            } else if normalized.chars().nth(512).is_some() {
                let preview: String = normalized.chars().take(512).collect();
                format!("{}...", preview)
            } else {
                normalized
            }
        })
        .unwrap_or_else(|| "-".to_string())
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
