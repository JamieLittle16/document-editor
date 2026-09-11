#![doc = "Standalone Slint qualification shell for Office R0A UI-framework evaluation."]

use slint::{ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};

slint::slint! {
    import { Button, LineEdit, ScrollView } from "std-widgets.slint";

    export component EditorShell inherits Window {
        title: "Office UI Qualification";
        width: 1100px;
        height: 800px;

        in property <image> document-tile;
        in property <[int]> page-indices;
        in-out property <string> search-text;
        in-out property <string> status-text: "Ready — authority-safe render preview";
        in-out property <int> zoom-percent: 100;
        in-out property <int> visible-page-index: 0;

        MenuBar {
            Menu {
                title: "File";
                MenuItem {
                    title: "New";
                    shortcut: @keys(Control + N);
                    activated => { root.status-text = "New document command reached shell"; }
                }
                MenuItem {
                    title: "Save";
                    shortcut: @keys(Control + S);
                    activated => { root.status-text = "Save command reached shell"; }
                }
            }
            Menu {
                title: "Edit";
                MenuItem {
                    title: "Find";
                    shortcut: @keys(Control + F);
                    activated => { search-input.focus(); }
                }
            }
            Menu {
                title: "View";
                MenuItem {
                    title: "Zoom In";
                    shortcut: @keys(Control + Plus);
                    activated => {
                        if root.zoom-percent < 200 {
                            root.zoom-percent += 10;
                        }
                    }
                }
                MenuItem {
                    title: "Zoom Out";
                    shortcut: @keys(Control + HyphenMinus);
                    activated => {
                        if root.zoom-percent > 50 {
                            root.zoom-percent -= 10;
                        }
                    }
                }
            }
        }

        VerticalLayout {
            spacing: 0px;

            Rectangle {
                height: 52px;
                background: #f5f6f8;
                accessible-role: search;
                accessible-id: "document-search-region";
                accessible-label: "Document search";

                HorizontalLayout {
                    padding-left: 12px;
                    padding-right: 12px;
                    padding-top: 8px;
                    padding-bottom: 8px;
                    spacing: 8px;

                    search-input := LineEdit {
                        text <=> root.search-text;
                        placeholder-text: "Find in document";
                        input-type: search;
                        accessible-id: "document-search-input";
                        horizontal-stretch: 1;
                    }

                    Button {
                        text: "Clear";
                        accessible-id: "document-search-clear";
                        clicked => { root.search-text = ""; }
                    }
                }
            }

            Rectangle {
                vertical-stretch: 1;
                background: #dfe3e8;
                accessible-role: main;
                accessible-id: "document-viewport";
                accessible-label: "Document viewport";

                ScrollView {
                    width: parent.width;
                    height: parent.height;
                    content-width: 1200px;
                    content-height: 64px + root.page-indices.length * (1088px * root.zoom-percent / 100);
                    content-y: -root.visible-page-index * (1088px * root.zoom-percent / 100);

                    Rectangle {
                        width: 1200px;
                        height: 64px + root.page-indices.length * (1088px * root.zoom-percent / 100);
                        background: #dfe3e8;

                        for page-index in root.page-indices: Rectangle {
                            x: 192px;
                            y: 64px + page-index * (1088px * root.zoom-percent / 100);
                            width: 816px * root.zoom-percent / 100;
                            height: 1056px * root.zoom-percent / 100;
                            background: white;
                            border-width: 1px;
                            border-color: #aeb5bd;
                            accessible-role: region;
                            accessible-label: "Rendered document page";

                            Image {
                                x: 0px;
                                y: 0px;
                                width: parent.width;
                                height: parent.height;
                                source: root.document-tile;
                                image-fit: fill;
                            }
                        }
                    }
                }
            }

            Rectangle {
                height: 34px;
                background: #f5f6f8;
                accessible-role: region;
                accessible-id: "document-status";
                accessible-label: "Document status";

                HorizontalLayout {
                    padding-left: 12px;
                    padding-right: 12px;
                    spacing: 8px;

                    Text {
                        text: root.status-text;
                        vertical-alignment: center;
                        horizontal-stretch: 1;
                    }
                    Text {
                        text: root.zoom-percent + "%";
                        vertical-alignment: center;
                    }
                }
            }
        }
    }
}

const TILE_WIDTH: u32 = 256;
const TILE_HEIGHT: u32 = 256;
const QUALIFICATION_PAGE_COUNT: i32 = 96;
const EXPECTED_TILE_BYTES: usize = (TILE_WIDTH as usize) * (TILE_HEIGHT as usize) * 4;
const EXPECTED_TILE_FNV1A64: u64 = 6_744_427_103_266_065_219;
const STRESS_TOTAL_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);
const STRESS_FRAME_LIMIT: std::time::Duration = std::time::Duration::from_secs(10);

fn qualification_tile() -> SharedPixelBuffer<Rgba8Pixel> {
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(TILE_WIDTH, TILE_HEIGHT);
    let bytes = buffer.make_mut_bytes();

    for y in 0..TILE_HEIGHT {
        for x in 0..TILE_WIDTH {
            let index = ((y * TILE_WIDTH + x) * 4) as usize;
            let in_page_body_x = (18..TILE_WIDTH - 18).contains(&x);
            let in_page_body_y = (18..TILE_HEIGHT - 18).contains(&y);
            let margin = !(in_page_body_x && in_page_body_y);
            let text_rule = !margin && y >= 42 && (y - 42) % 24 < 2 && x < 210;
            let rgba = if margin {
                [244, 244, 244, 255]
            } else if text_rule {
                [205, 212, 220, 255]
            } else {
                [255, 255, 255, 255]
            };
            bytes[index..index + 4].copy_from_slice(&rgba);
        }
    }

    buffer
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(1_469_598_103_934_665_603_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(1_099_511_628_211)
        })
}

fn run_snapshot_stress(ui: &EditorShell) {
    let steps = [
        (0, 70),
        (12, 100),
        (24, 125),
        (48, 150),
        (72, 200),
        (95, 100),
        (48, 80),
        (0, 100),
    ];
    let physical_size = ui.window().size();
    let expected_snapshot_bytes =
        usize::try_from(physical_size.width * physical_size.height).expect("window area must fit usize") * 4;
    let total_started = std::time::Instant::now();
    let mut slowest = std::time::Duration::ZERO;
    let mut checksum_xor = 0_u64;

    for (page_index, zoom_percent) in steps {
        ui.set_visible_page_index(page_index);
        ui.set_zoom_percent(zoom_percent);

        let frame_started = std::time::Instant::now();
        let snapshot = ui
            .window()
            .take_snapshot()
            .expect("software renderer must support native-window snapshots");
        let frame_elapsed = frame_started.elapsed();
        slowest = slowest.max(frame_elapsed);

        assert_eq!(snapshot.width(), physical_size.width);
        assert_eq!(snapshot.height(), physical_size.height);
        assert_eq!(snapshot.as_bytes().len(), expected_snapshot_bytes);
        checksum_xor ^= fnv1a64(snapshot.as_bytes());
        assert!(frame_elapsed < STRESS_FRAME_LIMIT);
    }

    let total_elapsed = total_started.elapsed();
    assert!(total_elapsed < STRESS_TOTAL_LIMIT);
    println!("ui_stress_pages={QUALIFICATION_PAGE_COUNT}");
    println!("ui_stress_snapshots={}", steps.len());
    println!("ui_stress_total_ms={}", total_elapsed.as_millis());
    println!("ui_stress_slowest_ms={}", slowest.as_millis());
    println!("ui_stress_snapshot_checksum_xor={checksum_xor}");
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = EditorShell::new()?;
    let tile = qualification_tile();
    let checksum = fnv1a64(tile.as_bytes());
    ui.set_document_tile(Image::from_rgba8(tile));
    ui.set_page_indices(ModelRc::new(VecModel::from(
        (0..QUALIFICATION_PAGE_COUNT).collect::<Vec<_>>(),
    )));

    if std::env::var_os("OFFICE_UI_QUALIFY_ONCE").is_some() {
        ui.show()?;
        let weak_ui = ui.as_weak();
        let stress_enabled = std::env::var_os("OFFICE_UI_STRESS").is_some();
        slint::Timer::single_shot(std::time::Duration::from_millis(50), move || {
            let ui = weak_ui
                .upgrade()
                .expect("qualification window must stay alive during native smoke");
            let scale_factor = ui.window().scale_factor();
            let physical_size = ui.window().size();
            println!("ui_framework=slint-1.17.1");
            println!("ui_backend=winit-software");
            println!("ui_accessibility=enabled");
            println!("ui_scale_factor={scale_factor}");
            println!(
                "ui_physical_size={}x{}",
                physical_size.width, physical_size.height
            );
            println!("ui_tile_bytes={EXPECTED_TILE_BYTES}");
            println!("ui_tile_checksum={checksum}");
            assert!(scale_factor.is_finite() && scale_factor > 0.0);
            assert!(physical_size.width > 0 && physical_size.height > 0);
            assert_eq!(checksum, EXPECTED_TILE_FNV1A64);

            if stress_enabled {
                run_snapshot_stress(&ui);
            }

            slint::quit_event_loop().expect("qualification event loop must be stoppable");
        });
        slint::run_event_loop()?;
        ui.hide()?;
        return Ok(());
    }

    ui.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualification_tile_matches_render_transfer_data_class() {
        let first = qualification_tile();
        let second = qualification_tile();

        assert_eq!(first.width(), TILE_WIDTH);
        assert_eq!(first.height(), TILE_HEIGHT);
        assert_eq!(first.as_bytes().len(), EXPECTED_TILE_BYTES);
        assert_eq!(first.as_bytes(), second.as_bytes());
        assert_eq!(fnv1a64(first.as_bytes()), EXPECTED_TILE_FNV1A64);
    }

    #[test]
    fn qualification_page_model_is_large_and_bounded() {
        assert_eq!(QUALIFICATION_PAGE_COUNT, 96);
        assert!(QUALIFICATION_PAGE_COUNT > 50);
        assert!(QUALIFICATION_PAGE_COUNT < 1_000);
    }
}
