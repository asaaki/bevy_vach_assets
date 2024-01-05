// no idea why this is needed here, probably a workspace issue
extern crate bevy;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            bevy_vach_assets::BevyVachAssetsPlugin {
                // note: adjust the paths to your needs!
                public_key_bytes: Some(include_bytes!("../../../secrets/key.pub")),

                // note: wasm does not support standard file I/O,
                //       so you have to provide the asset archive as a byte array;
                //       easiest way is to use include_bytes!() as shown below
                #[cfg(not(target_arch = "wasm32"))]
                static_archive: None,
                #[cfg(target_arch = "wasm32")]
                // note: adjust the paths to your needs!
                static_archive: Some(include_bytes!("../../../assets.bva")),
            },
            DefaultPlugins,
        ))
        .add_systems(Update, hello_world_system)
        .run();
}

fn hello_world_system() {
    println!("hello world");
}
