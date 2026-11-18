//! `nav::Route` derive: path round-trips, deep links, param parsing.

use egui_mineral::nav::{Route, RouteError, Router};

#[derive(Clone, Debug, PartialEq, Route)]
enum AppRoute {
    #[route("/")]
    Home,
    #[route("/user/{id}")]
    User { id: u32 },
    #[route("/user/{id}/posts/{tag?}")]
    Posts { id: u32, tag: Option<String> },
}

#[test]
fn roundtrip_and_deep_link() {
    for r in [
        AppRoute::Home,
        AppRoute::User { id: 7 },
        AppRoute::Posts { id: 7, tag: None },
        AppRoute::Posts {
            id: 7,
            tag: Some("a b/c".into()),
        },
    ] {
        assert_eq!(AppRoute::parse(&r.path()), Ok(r.clone()), "{}", r.path());
    }
    assert!(matches!(
        AppRoute::parse("/user/x"),
        Err(RouteError::InvalidParam { field: "id", .. })
    ));
    assert!(matches!(
        AppRoute::parse("/user/1/extra"),
        Err(RouteError::NoMatch { .. })
    ));

    let mut router = Router::new(AppRoute::Home);
    router.deep_link("/user/3/posts/rust?ref=mail").unwrap();
    assert_eq!(
        router.stack().cloned().collect::<Vec<_>>(),
        [
            AppRoute::Home,
            AppRoute::User { id: 3 },
            AppRoute::Posts { id: 3, tag: None },
            AppRoute::Posts {
                id: 3,
                tag: Some("rust".into())
            }
        ]
    );
    assert!(router.deep_link("/nope").is_err());
    assert_eq!(router.depth(), 4);
}

#[test]
fn display_and_from_str_round_trip() {
    for r in [
        AppRoute::Home,
        AppRoute::User { id: 7 },
        AppRoute::Posts {
            id: 7,
            tag: Some("a b/c".into()),
        },
    ] {
        assert_eq!(r.to_string(), r.path());
        assert_eq!(r.to_string().parse::<AppRoute>(), Ok(r));
    }
    assert!(matches!("/nope".parse::<AppRoute>(), Err(RouteError::NoMatch { .. })));
}

#[derive(Clone, Debug, PartialEq, Route)]
enum FileRoute {
    #[route("/files/{path}/{rev}")]
    File { path: String, rev: u32 },
}

#[test]
fn param_named_path_parses() {
    let r = FileRoute::File {
        path: "a".into(),
        rev: 2,
    };
    assert_eq!(FileRoute::parse(&r.path()), Ok(r));
}

#[test]
fn empty_and_tilde_params_round_trip() {
    for path in ["", "~", "~x", "~~", "a~", "/", "a b"] {
        let r = FileRoute::File {
            path: path.into(),
            rev: 2,
        };
        assert_eq!(FileRoute::parse(&r.path()), Ok(r.clone()), "{path:?} via {}", r.path());
    }
    let empty = FileRoute::File {
        path: String::new(),
        rev: 2,
    };
    assert_eq!(empty.path(), "/files/~/2");
    let tag = AppRoute::Posts {
        id: 1,
        tag: Some(String::new()),
    };
    assert_eq!(AppRoute::parse(&tag.path()), Ok(tag));
}

#[test]
fn router_back_replace_and_persistence() {
    let mut router = Router::new(AppRoute::Home);
    assert!(!router.back());
    assert_eq!(router.stack().collect::<Vec<_>>(), [&AppRoute::Home]);

    router.push(AppRoute::User { id: 1 });
    router.replace(AppRoute::User { id: 2 });
    assert_eq!(
        router.stack().collect::<Vec<_>>(),
        [&AppRoute::Home, &AppRoute::User { id: 2 }]
    );

    let restored = Router::<AppRoute>::from_paths(&router.to_paths()).unwrap();
    assert!(restored.stack().eq(router.stack()));
    assert!(router.back() && !router.back());

    assert!(matches!(
        Router::<AppRoute>::from_paths(&[]),
        Err(RouteError::EmptyStack)
    ));
}
