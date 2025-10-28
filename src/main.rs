mod components;

use gpui::{AppContext, Application, WindowOptions};
use gpui_component::Root;

use crate::components::instance::{Instance, InstanceGrid, InstanceStatus, ModLoader};

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let grid = cx.new(|cx| {
                    let instance1 = cx.new(|_| {
                        Instance::new(1, "Minecraft 1.20.1".to_string())
                            .version("1.20.1")
                            .modloader(ModLoader::Vanilla)
                            .last_played("2 hours ago")
                            .play_time(42.5)
                            .status(InstanceStatus::Ready)
                            .on_play(|| println!("Playing Minecraft 1.20.1"))
                            .on_settings(|| println!("Settings for Minecraft 1.20.1"))
                            .on_delete(|| println!("Delete Minecraft 1.20.1"))
                    });

                    let instance2 = cx.new(|_| {
                        Instance::new(2, "Modded Fabric".to_string())
                            .version("1.20.4")
                            .modloader(ModLoader::Fabric)
                            .last_played("Yesterday")
                            .play_time(128.3)
                            .status(InstanceStatus::Ready)
                            .on_play(|| println!("Playing Modded Fabric"))
                            .on_settings(|| println!("Settings for Modded Fabric"))
                            .on_delete(|| println!("Delete Modded Fabric"))
                    });

                    let instance3 = cx.new(|_| {
                        Instance::new(3, "Forge Modpack".to_string())
                            .version("1.19.2")
                            .modloader(ModLoader::Forge)
                            .last_played("3 days ago")
                            .play_time(67.8)
                            .status(InstanceStatus::Running)
                            .on_play(|| println!("Stop Forge Modpack"))
                            .on_settings(|| println!("Settings for Forge Modpack"))
                            .on_delete(|| println!("Delete Forge Modpack"))
                    });

                    let instance4 = cx.new(|_| {
                        Instance::new(4, "New Instance".to_string())
                            .version("1.21")
                            .modloader(ModLoader::Quilt)
                            .play_time(0.0)
                            .status(InstanceStatus::Installing)
                            .on_settings(|| println!("Settings for New Instance"))
                            .on_delete(|| println!("Delete New Instance"))
                    });

                    InstanceGrid::new()
                        .add_instance(instance1.into())
                        .add_instance(instance2.into())
                        .add_instance(instance3.into())
                        .add_instance(instance4.into())
                });

                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(grid.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
