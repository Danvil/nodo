// Copyright 2022 by David Weikersdorfer

pub mod nodo {
    pub mod inspector {
        pub use inspector_proto::*;
    }
}

pub mod error_log;
pub mod node_ascii_art_tui_widget;
pub mod nodo_app_link;
pub mod tui_app_state;
pub mod tui_style;
pub mod tui_widget_selection;
pub mod widget_home;
pub mod widget_manifold;
pub mod widget_statistics;
