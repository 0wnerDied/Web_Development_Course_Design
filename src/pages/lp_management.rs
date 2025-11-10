use crate::api;
use crate::components::use_current_user;
use crate::models::UserLpSummary;
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::collections::HashMap;

#[component]
pub fn LpManagement() -> Element {
    let mut lp_logs = use_signal(Vec::<api::LpLog>::new);
    let mut lp_types = use_signal(HashMap::<i64, String>::new);
    let mut search_user = use_signal(String::new);
    let mut selected_summary = use_signal(|| None::<UserLpSummary>);
    let mut selected_history = use_signal(Vec::<api::LpLog>::new);
    let mut lp_summaries = use_signal(Vec::<UserLpSummary>::new);
    let mut show_pending_only = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let loading_visible = use_signal(|| false);
    let current_user = use_current_user();
    let mut selected_ids = use_signal(Vec::<i64>::new);

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
        spawn(async move {
            loading.set(true);

            match api::get_lp_types().await {
                Ok(types) => {
                    let map = types
                        .into_iter()
                        .filter_map(|tp| tp.id.map(|id| (id, tp.name)))
                        .collect::<HashMap<_, _>>();
                    lp_types.set(map);
                }
                Err(e) => {
                    error.set(Some(format!("加载LP类型失败: {}", e)));
                }
            }

            match api::get_lp_logs().await {
                Ok(mut logs) => {
                    if *show_pending_only.peek() {
                        logs.retain(|log| log.status == 0);
                    }
                    lp_logs.set(logs);
                }
                Err(e) => {
                    error.set(Some(format!("加载失败: {}", e)));
                }
            }

            loading.set(false);
        });
    };

    let mut query_user_lp = move || {
        let raw = search_user.read().clone();
        let qq = raw.trim().to_string();
        if qq.is_empty() {
            error.set(Some("请输入要查询的QQ号".to_string()));
            selected_summary.set(None);
            selected_history.set(Vec::new());
            return;
        }

        search_user.set(qq.clone());
        success.set(None);
        spawn(async move {
            loading.set(true);
            match api::get_user_lp_detail(&qq).await {
                Ok(detail) => {
                    selected_summary.set(detail.summary);
                    selected_history.set(detail.history);
                    error.set(None);
                    success.set(Some(format!("已加载 {qq} 的LP记录")));
                }
                Err(e) => {
                    error.set(Some(format!("查询失败: {}", e)));
                    success.set(None);
                    selected_summary.set(None);
                    selected_history.set(Vec::new());
                }
            }
            loading.set(false);
        });
    };

    let mut clear_query = move || {
        search_user.set(String::new());
        selected_summary.set(None);
        selected_history.set(Vec::new());
        lp_summaries.set(Vec::new());
        success.set(None);
        error.set(None);
    };

    let mut load_summaries = move || {
        success.set(None);
        spawn(async move {
            loading.set(true);
            match api::get_lp_summaries().await {
                Ok(data) => {
                    let total = data.len();
                    lp_summaries.set(data);
                    error.set(None);
                    if total > 0 {
                        success.set(Some(format!("已加载 {total} 条LP汇总")));
                    } else {
                        success.set(Some("暂无LP汇总数据".to_string()));
                    }
                }
                Err(e) => {
                    error.set(Some(format!("获取LP汇总失败: {}", e)));
                    success.set(None);
                }
            }
            loading.set(false);
        });
    };

    let mut process_lp = move |id: i64, status: i32| {
        let session_user = current_user.read().clone();
        let Some(user) = session_user else {
            error.set(Some("请先登录后再审批LP".to_string()));
            return;
        };

        if !user.permissions.contains(&"审核LP".to_string()) {
            error.set(Some("当前账号无审批权限".to_string()));
            return;
        }

        spawn(async move {
            loading.set(true);
            match api::process_lp(id, status).await {
                Ok(msg) => {
                    success.set(Some(msg));
                    error.set(None);
                    load_logs();
                }
                Err(e) => {
                    error.set(Some(format!("处理失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 批量处理LP
    let mut batch_process_lp = move |status: i32| {
        let session_user = current_user.read().clone();
        let Some(user) = session_user else {
            error.set(Some("请先登录后再审批LP".to_string()));
            return;
        };

        if !user.permissions.contains(&"审核LP".to_string()) {
            error.set(Some("当前账号无审批权限".to_string()));
            return;
        };

        let ids = selected_ids.read().clone();
        if ids.is_empty() {
            error.set(Some("请先选择要批量处理的申请".to_string()));
            return;
        }

        spawn(async move {
            loading.set(true);
            match api::batch_process_lp(ids.clone(), status).await {
                Ok(resp) => {
                    success.set(Some(format!(
                        "批量处理成功: 请求{}条，通过{}条",
                        resp.requested_count, resp.approved_count
                    )));
                    error.set(None);
                    selected_ids.set(Vec::new()); // 清空选中
                    load_logs();
                }
                Err(e) => {
                    error.set(Some(format!("批量处理失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 切换单个选择
    let mut toggle_select = move |id: i64| {
        let mut ids = selected_ids.read().clone();
        if let Some(pos) = ids.iter().position(|x| *x == id) {
            ids.remove(pos);
        } else {
            ids.push(id);
        }
        selected_ids.set(ids);
    };

    // 全选/取消全选
    let mut toggle_select_all = move || {
        let logs = lp_logs.read().clone();
        let all_ids: Vec<i64> = logs
            .iter()
            .filter(|log| log.status == 0) // 只选择待审核的
            .filter_map(|log| log.id)
            .collect();

        let current_selected = selected_ids.read().clone();
        if current_selected.len() == all_ids.len() {
            selected_ids.set(Vec::new()); // 取消全选
        } else {
            selected_ids.set(all_ids); // 全选
        }
    };

    use_effect(move || {
        load_logs();
    });

    let type_map_snapshot = lp_types.read().clone();
    let logs_snapshot = lp_logs.read().clone();
    let summary_snapshot = selected_summary.read().clone();
    let history_snapshot = selected_history.read().clone();
    let summaries_snapshot = lp_summaries.read().clone();
    let error_snapshot = error.read().clone();
    let is_loading = *loading.read();
    let searched_user_trimmed = search_user.read().trim().to_string();
    let resolve_type = |lp_type: i64| -> String {
        type_map_snapshot
            .get(&lp_type)
            .cloned()
            .unwrap_or_else(|| format!("类型#{}", lp_type))
    };

    rsx! {
        div { class: "page-container",
            h1 { "LP管理" }

            div { class: "toolbar",
                Link { to: crate::Route::LpSubmit {},
                    button { class: "btn-primary", "提交LP申请" }
                }

                label { class: "checkbox-label",
                    input {
                        r#type: "checkbox",
                        checked: *show_pending_only.read(),
                        disabled: *loading.read(),
                        onchange: move |evt| {
                            show_pending_only.set(evt.checked());
                            load_logs();
                        }
                    }
                    " 只显示待处理"
                }

                button {
                    class: "btn-secondary",
                    onclick: move |_| load_logs(),
                    disabled: *loading.read(),
                    "刷新"
                }
            }

            div { class: "toolbar",
                div { class: "search-box",
                    input {
                        r#type: "text",
                        placeholder: "输入QQ号查询LP记录",
                        value: "{search_user}",
                        oninput: move |evt| search_user.set(evt.value().clone()),
                        disabled: *loading.read()
                    }
                    button {
                        class: "btn-info",
                        onclick: move |_| query_user_lp(),
                        disabled: *loading.read(),
                        "查询"
                    }
                    button {
                        class: "btn-secondary",
                        onclick: move |_| clear_query(),
                        disabled: *loading.read(),
                        "清除"
                    }
                    button {
                        class: "btn-primary",
                        onclick: move |_| load_summaries(),
                        disabled: *loading.read(),
                        "加载LP汇总"
                    }
                }
            }

            if *loading_visible.read() {
                div { class: "loading-message", "加载中..." }
            }

            if let Some(err) = error_snapshot.as_ref() {
                div { class: "error-message", "{err}" }
            }

            if let Some(succ) = success.read().as_ref() {
                div { class: "success-message", "{succ}" }
            }

            if let Some(summary) = summary_snapshot.as_ref() {
                div { class: "info-section summary-section",
                    h2 { "用户LP汇总" }
                    p { class: "summary-subtitle", "{summary.nickname} ({summary.qq})" }

                    div { class: "summary-grid",
                        div { class: "summary-item",
                            span { "总LP" }
                            strong { "{summary.total_lp}" }
                        }
                        div { class: "summary-item",
                            span { "待审核" }
                            strong { "{summary.pending_count}" }
                        }
                        div { class: "summary-item",
                            span { "已通过" }
                            strong { "{summary.approved_count}" }
                        }
                        div { class: "summary-item",
                            span { "已拒绝" }
                            strong { "{summary.rejected_count}" }
                        }
                    }
                }
            } else if !searched_user_trimmed.is_empty()
                && history_snapshot.is_empty()
                && error_snapshot.is_none()
                && !is_loading
            {
                div { class: "info-section summary-section",
                    p { "未找到 {searched_user_trimmed} 的LP数据" }
                }
            }

            if !history_snapshot.is_empty() {
                div { class: "table-container",
                    //h2 { class: "section-title", "用户LP记录" }
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "ID" }
                                th { "上传时间" }
                                th { "关联用户" }
                                th { "类型" }
                                th { "数量" }
                                th { "状态" }
                                th { "处理人" }
                                th { "处理时间" }
                            }
                        }
                        tbody {
                            for log in history_snapshot.iter() {
                                tr {
                                    key: "detail-{log.id.unwrap_or_default()}",
                                    td { "{log.id.unwrap_or_default()}" }
                                    td { "{log.upload_time}" }
                                    td { "{log.user_qq}" }
                                    td { "{resolve_type(log.lp_type)}" }
                                    td {
                                        class: if log.num > 0 { "text-success" } else if log.num < 0 { "text-danger" } else { "" },
                                        "{log.num}"
                                    }
                                    td {
                                        match log.status {
                                            0 => rsx!(span { class: "badge badge-warning", "待处理" }),
                                            1 => rsx!(span { class: "badge badge-success", "已通过" }),
                                            2 => rsx!(span { class: "badge badge-danger", "已拒绝" }),
                                            _ => rsx!(span { class: "badge", "未知" }),
                                        }
                                    }
                                    td {
                                        if let Some(processor) = &log.process_user_qq {
                                            "{processor}"
                                        } else {
                                            "-"
                                        }
                                    }
                                    td {
                                        if let Some(process_time) = &log.process_time {
                                            "{process_time}"
                                        } else {
                                            "-"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !summaries_snapshot.is_empty() {
                div { class: "table-container",
                    //h2 { class: "section-title", style: "text-align: center;", "LP汇总排行" }
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "排名" }
                                th { "QQ号" }
                                th { "昵称" }
                                th { "总LP" }
                                th { "待审核" }
                                th { "已通过" }
                                th { "已拒绝" }
                            }
                        }
                        tbody {
                            for (index, summary) in summaries_snapshot.iter().enumerate() {
                                tr {
                                    key: "summary-{summary.qq}",
                                    td { "{index + 1}" }
                                    td { "{summary.qq}" }
                                    td { "{summary.nickname}" }
                                    td { "{summary.total_lp}" }
                                    td { "{summary.pending_count}" }
                                    td { "{summary.approved_count}" }
                                    td { "{summary.rejected_count}" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "table-container",
                // 批量操作按钮
                div { class: "batch-actions",
                    button {
                        class: "btn-secondary",
                        onclick: move |_| toggle_select_all(),
                        "全选/取消全选"
                    }
                    button {
                        class: "btn-primary",
                        onclick: move |_| batch_process_lp(1),
                        disabled: *loading.read() || selected_ids.read().is_empty(),
                        "批量通过 ({selected_ids.read().len()})"
                    }
                    button {
                        class: "btn-danger",
                        onclick: move |_| batch_process_lp(2),
                        disabled: *loading.read() || selected_ids.read().is_empty(),
                        "批量拒绝 ({selected_ids.read().len()})"
                    }
                }

                table { class: "data-table",
                    thead {
                        tr {
                            th { "选择" }
                            th { "ID" }
                            th { "上传时间" }
                            th { "上传者" }
                            th { "关联用户" }
                            th { "角色" }
                            th { "类型" }
                            th { "数量" }
                            th { "原因" }
                            th { "状态" }
                            th { "处理人" }
                            th { "处理时间" }
                            th { "操作" }
                        }
                    }
                    tbody {
                        for log in logs_snapshot.iter() {
                            {
                                let log_id = log.id.unwrap_or_default();
                                let is_selected = selected_ids.read().contains(&log_id);
                                let is_pending = log.status == 0;

                                rsx! {
                                    tr {
                                        key: "{log_id}",
                                        td {
                                            if is_pending {
                                                input {
                                                    r#type: "checkbox",
                                                    checked: is_selected,
                                                    onchange: move |_| toggle_select(log_id),
                                                }
                                            }
                                        }
                                        td { "{log_id}" }
                                        td { "{log.upload_time}" }
                                        td { "{log.upload_user_qq}" }
                                        td { "{log.user_qq}" }
                                td {
                                    if let Some(role) = &log.role {
                                        "{role}"
                                    } else {
                                        "-"
                                    }
                                }
                                td { "{resolve_type(log.lp_type)}" }
                                td {
                                    class: if log.num > 0 { "text-success" } else if log.num < 0 { "text-danger" } else { "" },
                                    "{log.num}"
                                }
                                td { "{log.reason}" }
                                td {
                                    match log.status {
                                        0 => rsx!(span { class: "badge badge-warning", "待处理" }),
                                        1 => rsx!(span { class: "badge badge-success", "已通过" }),
                                        2 => rsx!(span { class: "badge badge-danger", "已拒绝" }),
                                        _ => rsx!(span { class: "badge", "未知" }),
                                    }
                                }
                                td {
                                    if let Some(processor) = &log.process_user_qq {
                                        "{processor}"
                                    } else {
                                        "-"
                                    }
                                }
                                td {
                                    if let Some(process_time) = &log.process_time {
                                        "{process_time}"
                                    } else {
                                        "-"
                                    }
                                }
                                        td {
                                            if log.status == 0 {
                                                if let Some(id_val) = log.id {
                                                    button {
                                                        class: "btn-small btn-success",
                                                        onclick: move |_| process_lp(id_val, 1),
                                                        disabled: *loading.read(),
                                                        "通过"
                                                    }
                                                    button {
                                                        class: "btn-small btn-danger",
                                                        onclick: move |_| process_lp(id_val, 2),
                                                        disabled: *loading.read(),
                                                        "拒绝"
                                                    }
                                                } else {
                                                    span { class: "text-warning", "记录缺少ID" }
                                                }
                                            } else {
                                                "-"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
