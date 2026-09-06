#![doc = "Standalone Slint qualification shell for Office R0A UI-framework evaluation."]

use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

slint::slint! {
    import { Button, LineEdit, ScrollView } from "std-widgets.slint";

    export component EditorShell inherits Window {
        title: "Office UI Qualification";
        width: 1100px;
        height: 800px;
        min-width: 720px;
        min-height: 520px;

        in property <image> document-tile;
        in-out property <string> search-text;
        in-out property <string> status-text: "Ready — authority-safe render preview";
        in-out property <int> zoom-percent: 100;

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
                    shortcut: @keys(Control + Minus);
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
                    viewport-width: 1200px;
                    viewport-height: 1500px;

                    Rectangle {
                        width: 1200px;
                        height: 1500px;
                        background: #dfe3e8;

                        page := Rectangle {
                            x: 192px;
                            y: 64px;
                            width: 816px * root.zoom-percent / 100;
                            height: 1056px * root.zoom-percent / 100;
                            background: white;
                            border-width: 1px;
                            border-color: #aeb5bd;
                            accessible-role: region;
                            accessible-id: "rendered-document-page";
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
const EXPECTED_TILE_BYTES: usize = (TILE_WIDTH as usize) * (TILE_HEIGHT as usize) * 4;
const EXPECTED_TILE_FNV1A64: u64 = 6_744_427_103_266_065_219;

fn qualification_tile() -> SharedPixelBuffer<Rgba8Pixel> {
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(TILE_WIDTH, TILE_HEIGHT);
    let bytes = buffer.make_mut_bytes();

    for y in 0..TILE_HEIGHT {
        for x in 0..TILE_WIDTH {
            let index = ((y * TILE_WIDTH + x) * 4) as usize;
            let margin = x < 18 || x >= TILE_WIDTH - 18 || y < 18 || y >= TILE_HEIGHT - 18;
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
    bytes.iter().fold(1_469_598_103_934_665_603_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(1_099_511_628_211)
    })
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = EditorShell::new()?;
    let tile = qualification_tile();
    let checksum = fnv1a64(tile.as_bytes());
    ui.set_document_tile(Image::from_rgba8(tile));

    if std::env::var_os("OFFICE_UI_QUALIFY_ONCE").is_some() {
        ui.show()?;
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
}
