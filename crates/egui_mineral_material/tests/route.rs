//! `Route` derive honors `#[route(crate = ..)]`.

use egui_mineral_material::egui_mineral::nav as core_nav;

// Facade whose `__private::segment` is observable; the derive must expand through it.
mod facade {
    pub(crate) mod nav {
        pub(crate) use super::super::core_nav::{Route, RouteError};

        pub(crate) mod __private {
            pub(crate) use super::super::super::core_nav::__private::*;

            pub(crate) fn segment(field: &str, value: &impl std::fmt::Display) -> String {
                format!("<{}>", super::super::super::core_nav::__private::segment(field, value))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, core_nav::Route)]
#[route(crate = facade)]
enum AppRoute {
    #[route("/")]
    Home,
    #[route("/user/{id}")]
    User { id: u32 },
}

#[test]
fn derive_expands_through_crate_override() {
    use core_nav::Route;
    assert_eq!(AppRoute::User { id: 3 }.path(), "/user/<3>");
    assert_eq!(AppRoute::Home.path(), "/");
}
