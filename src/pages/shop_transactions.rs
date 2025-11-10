use crate::api;
use crate::components::use_current_user;
use dioxus::prelude::*;

#[derive(Clone, Debug)]
enum TransactionType {
    Purchase(api::ShopLog),
    Sale(api::ShopLog),
}

#[component]
pub fn ShopTransactions() -> Element {
    let mut purchases = use_signal(Vec::<api::ShopLog>::new);
    let mut sales = use_signal(Vec::<api::ShopLog>::new);
    let mut all_transactions = use_signal(Vec::<TransactionType>::new);
    let mut error = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let mut current_page = use_signal(|| 1);
    let page_size = 10;
    let current_user = use_current_user();

    let mut load_transactions = move || {
        let session = current_user.read().clone();
        let Some(user) = session else {
            error.set(Some("请先登录后查看交易记录".to_string()));
            return;
        };

        let user_qq = user.qq.clone();

        spawn(async move {
            loading.set(true);
            match api::get_shop_transactions(&user_qq).await {
                Ok(transactions) => {
                    purchases.set(transactions.purchases.clone());
                    sales.set(transactions.sales.clone());

                    // 合并并按时间排序
                    let mut merged: Vec<TransactionType> = Vec::new();
                    for purchase in transactions.purchases {
                        merged.push(TransactionType::Purchase(purchase));
                    }
                    for sale in transactions.sales {
                        merged.push(TransactionType::Sale(sale));
                    }

                    // 按时间降序排序（最新的在前面）
                    merged.sort_by(|a, b| {
                        let time_a = match a {
                            TransactionType::Purchase(log) => &log.time,
                            TransactionType::Sale(log) => &log.time,
                        };
                        let time_b = match b {
                            TransactionType::Purchase(log) => &log.time,
                            TransactionType::Sale(log) => &log.time,
                        };
                        time_b.cmp(time_a) // 降序
                    });

                    all_transactions.set(merged);
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(format!("加载失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        load_transactions();
    });

    rsx! {
        div { class: "page-container",
            h1 { "我的交易记录" }

            div { class: "toolbar",
                button {
                    class: "btn-secondary",
                    onclick: move |_| load_transactions(),
                    disabled: *loading.read(),
                    "刷新"
                }
                Link { to: crate::Route::Shop {},
                    button { class: "btn-secondary", "返回商店" }
                }
            }

            if *loading.read() {
                div { class: "loading-message", "加载中..." }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "error-message", "{err}" }
            }

            div { class: "table-container",
                if all_transactions.read().is_empty() {
                    div { class: "empty-state", "暂无交易记录" }
                } else {
                    {
                        let total = all_transactions.read().len();
                        let total_pages = total.div_ceil(page_size);
                        let current = *current_page.read();
                        let start = (current - 1) * page_size;

                        rsx! {
                            table { class: "data-table",
                                thead {
                                    tr {
                                        th { "类型" }
                                        th { "商品名称" }
                                        th { "数量" }
                                        th { "价格" }
                                        th { "交易对方" }
                                        th { "地点" }
                                        th { "交易时间" }
                                    }
                                }
                                tbody {
                                    for (idx, transaction) in all_transactions.read().iter().enumerate().skip(start).take(page_size) {
                                        {
                                            match transaction {
                                                TransactionType::Purchase(purchase) => rsx! {
                                                    tr {
                                                        key: "tx-{idx}",
                                                        td {
                                                            span { class: "badge badge-info", "购买" }
                                                        }
                                                        td { "{purchase.name}" }
                                                        td { "{purchase.count}" }
                                                        td { "{purchase.price} 元" }
                                                        td { "{purchase.seller}" }
                                                        td { "{purchase.location}" }
                                                        td { "{purchase.time}" }
                                                    }
                                                },
                                                TransactionType::Sale(sale) => rsx! {
                                                    tr {
                                                        key: "tx-{idx}",
                                                        td {
                                                            span { class: "badge badge-success", "销售" }
                                                        }
                                                        td { "{sale.name}" }
                                                        td { "{sale.count}" }
                                                        td { "{sale.price} 元" }
                                                        td { "{sale.buyer}" }
                                                        td { "{sale.location}" }
                                                        td { "{sale.time}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // 分页控制
                            if total_pages > 1 {
                                div { class: "pagination",
                                    button {
                                        class: "btn-small btn-secondary",
                                        disabled: current == 1,
                                        onclick: move |_| {
                                            if current > 1 {
                                                current_page.set(current - 1);
                                            }
                                        },
                                        "上一页"
                                    }

                                    span { class: "page-info",
                                        "第 {current} / {total_pages} 页"
                                    }

                                    button {
                                        class: "btn-small btn-secondary",
                                        disabled: current >= total_pages,
                                        onclick: move |_| {
                                            if current < total_pages {
                                                current_page.set(current + 1);
                                            }
                                        },
                                        "下一页"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "stats",
                "共 {purchases.read().len()} 条购买记录，{sales.read().len()} 条销售记录（共 {all_transactions.read().len()} 条）"
            }
        }
    }
}
