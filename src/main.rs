#![allow(non_snake_case)]

mod components;
mod models;
mod pages;

#[cfg(feature = "frontend")]
mod api;

#[cfg(feature = "backend")]
mod db;

use components::AppContext;
use dioxus::logger::tracing::{info, warn, Level};
use dioxus::prelude::*;
use dioxus_router::hooks::use_route;
use models::SessionUser;
use pages::*;

fn main() {
    // 初始化日志
    dioxus_logger::init(Level::INFO).expect("日志初始化失败");
    info!("团队运营管理系统启动中...");

    #[cfg(feature = "backend")]
    {
        // 后端模式:初始化数据库
        let _ = &*db::DB;
        info!("数据库初始化完成");
    }

    #[cfg(feature = "frontend")]
    {
        // 前端模式:直接启动Web应用
        info!("前端应用启动中...");
    }

    launch(App);
}

#[component]
fn App() -> Element {
    let current_user = use_signal(|| None::<SessionUser>);
    let is_loading = use_signal(|| true); // 添加加载状态
    use_context_provider(|| AppContext {
        current_user,
        is_loading,
    });

    #[cfg(feature = "frontend")]
    {
        let mut current_user_signal = current_user;
        let mut is_loading_signal = is_loading;
        let mut init_attempted = use_signal(|| false);
        use_effect(move || {
            if *init_attempted.read() {
                return;
            }
            init_attempted.set(true);

            if crate::api::get_token().is_some() {
                spawn(async move {
                    match crate::api::get_profile().await {
                        Ok(user) => {
                            current_user_signal.set(Some(SessionUser {
                                qq: user.qq.clone(),
                                nickname: user.nickname.clone(),
                                birthday: user.birthday.clone(),
                                main_role_id: None,
                                role_name: user.role_name.clone(),
                                permissions: user.permissions.clone(),
                            }));
                            is_loading_signal.set(false);
                        }
                        Err(_) => {
                            let _ = crate::api::clear_token();
                            is_loading_signal.set(false);
                        }
                    }
                });
            } else {
                // 没有token，直接完成加载
                is_loading_signal.set(false);
            }
        });
    }

    #[cfg(not(feature = "frontend"))]
    {
        // 后端模式不需要检查token
        is_loading.set(false);
    }

    rsx! {
        Router::<Route> {}
    }
}

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Layout)]
        #[route("/")]
        Home {},
        #[route("/login")]
        Login {},
        #[route("/register")]
        Register {},
        #[route("/users")]
        Users {},
        #[route("/roles")]
        Roles {},
        #[route("/lp")]
        LpManagement {},
        #[route("/lp/submit")]
        LpSubmit {},
        #[route("/luckydraw")]
        LuckyDraw {},
        #[route("/shop")]
        Shop {},
        #[route("/shop/my")]
        MyShop {},
        #[route("/shop/transactions")]
        ShopTransactions {},
        #[route("/profile")]
        Profile {},
        #[route("/logs")]
        Logs {},
}

#[component]
fn Layout() -> Element {
    let app_ctx = components::use_app_context();
    let mut current_user = app_ctx.current_user;
    let is_loading = app_ctx.is_loading;
    let nav = use_navigator();
    let current_route: Route = use_route();
    let is_public_route = matches!(current_route, Route::Login {} | Route::Register {});
    let user_state = current_user.read().clone();

    // 如果正在加载，显示加载界面
    if *is_loading.read() {
        return rsx! {
            div { class: "page-container",
                style { {include_str!("../assets/main.css")} }
                div {
                    style: "display: flex; justify-content: center; align-items: center; height: 100vh;",
                    h2 { "加载中..." }
                }
            }
        };
    }

    // 加载完成后，如果访问需要认证的页面但未登录，则跳转到登录页
    if !is_public_route && user_state.is_none() {
        nav.replace(Route::Login {});
        return rsx! {
            div { class: "page-container",
                h1 { "正在跳转" }
                p { "请先登录后再访问系统功能。" }
            }
        };
    }

    rsx! {
        div { class: "app-container",
            style { {include_str!("../assets/main.css")} }

            nav { class: "navbar",
                div { class: "nav-brand",
                    "团队运营管理系统"
                }
                div { class: "nav-links",
                    if user_state.is_some() {
                        Link { to: Route::Home {}, "首页" }
                        Link { to: Route::Users {}, "用户管理" }
                        Link { to: Route::Roles {}, "角色管理" }
                        Link { to: Route::LpManagement {}, "LP管理" }
                        Link { to: Route::LuckyDraw {}, "抽奖活动" }
                        Link { to: Route::Shop {}, "虚拟商店" }
                        Link { to: Route::Logs {}, "系统日志" }
                    }

                    if let Some(user) = user_state.as_ref() {
                        span { class: "nav-user", "欢迎, {user.nickname}" }
                        Link { to: Route::Profile {}, "个人中心" }
                        button {
                            class: "btn-small btn-secondary",
                            onclick: move |_| {
                                #[cfg(feature = "frontend")]
                                if let Err(err) = crate::api::clear_token() {
                                    warn!("清理Token失败: {}", err);
                                }
                                current_user.set(None);
                                nav.replace(Route::Login {});
                            },
                            "退出"
                        }
                    } else {
                        Link { to: Route::Login {}, "登录" }
                        Link { to: Route::Register {}, "注册" }
                    }
                }
            }

            main { class: "main-content",
                Outlet::<Route> {}
            }

            footer { class: "footer",
                "© 2025 团队运营管理系统"
            }
        }
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        div { class: "page-container",
            h1 { "欢迎使用团队运营管理系统" }

            div { class: "feature-grid",
                div { class: "feature-card",
                    h3 { "用户管理" }
                    p { "注册、登录、用户信息管理" }
                    Link { to: Route::Users {},
                        button { class: "btn-primary", "进入" }
                    }
                }

                div { class: "feature-card",
                    h3 { "角色管理" }
                    p { "管理角色、分配用户权限" }
                    Link { to: Route::Roles {},
                        button { class: "btn-primary", "进入" }
                    }
                }

                div { class: "feature-card",
                    h3 { "LP管理" }
                    p { "LP申请、审批、查询" }
                    Link { to: Route::LpManagement {},
                        button { class: "btn-primary", "进入" }
                    }
                }

                div { class: "feature-card",
                    h3 { "抽奖活动" }
                    p { "创建抽奖、开奖、中奖记录" }
                    Link { to: Route::LuckyDraw {},
                        button { class: "btn-primary", "进入" }
                    }
                }

                div { class: "feature-card",
                    h3 { "虚拟商店" }
                    p { "商品上架、购买、交易记录" }
                    Link { to: Route::Shop {},
                        button { class: "btn-primary", "进入" }
                    }
                }

                div { class: "feature-card",
                    h3 { "系统日志" }
                    p { "查看系统请求日志和操作记录" }
                    Link { to: Route::Logs {},
                        button { class: "btn-primary", "进入" }
                    }
                }
            }
        }
    }
}
