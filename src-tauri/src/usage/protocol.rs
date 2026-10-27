// Bindings are generated at compile time from the vendored protocol XML.
// Both XML files carry the permissive MIT-style copyright reproduced inside
// them; this avoids depending on a GPL-licensed protocol crate.

macro_rules! protocol_bindings {
    ($name:ident, $path:expr, [$($imports:path),*], [$($ifaces:path),*]) => {
        pub mod $name {
            #![allow(
                dead_code,
                unused_imports,
                non_camel_case_types,
                non_snake_case,
                non_upper_case_globals,
                deprecated
            )]

            use wayland_client;
            use wayland_client::protocol::*;
            $(use $imports::*;)*

            pub mod __interfaces {
                use wayland_client::protocol::__interfaces::*;
                $(use $ifaces::*;)*
                wayland_scanner::generate_interfaces!($path);
            }
            use self::__interfaces::*;

            wayland_scanner::generate_client_code!($path);
        }
    };
}

pub mod cosmic {
    protocol_bindings!(
        workspace,
        "protocols/cosmic-workspace-unstable-v1.xml",
        [],
        []
    );
    protocol_bindings!(
        toplevel,
        "protocols/cosmic-toplevel-info-unstable-v1.xml",
        [
            wayland_protocols::ext::foreign_toplevel_list::v1::client,
            wayland_protocols::ext::workspace::v1::client,
            crate::usage::protocol::cosmic::workspace
        ],
        [
            wayland_protocols::ext::foreign_toplevel_list::v1::client::__interfaces,
            wayland_protocols::ext::workspace::v1::client::__interfaces,
            crate::usage::protocol::cosmic::workspace::__interfaces
        ]
    );
}
