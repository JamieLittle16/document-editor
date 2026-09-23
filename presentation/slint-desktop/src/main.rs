#![doc = "Provisional Slint desktop presentation adapter for the Office editor."]

use std::cell::RefCell;
use std::rc::Rc;

use app_core::{AppCore, EditorSnapshot};
use document_engine_mock::MockDocumentEngine;
use slint::{ComponentHandle, SharedString};

const STARTER_DOCUMENT: &str = "Office document\n\nStart typing here. Every user edit is committed through AppCore and the authoritative document session.";

slint::slint! {
    import { Button, LineEdit, TextEdit } from "std-widgets.slint";

    export component DesktopShell inherits Window {
        title: "Office";
        width: 1100px;
        height: 800px;
        default-font-size: 14px;

        in-out property <string> document-text;
        in property <string> status-text;
        in-out property <string> search-text;
        in-out property <int> zoom-percent: 100;

        callback document-edited(string);
        callback new-document();
        callback reset-document();

        MenuBar {
            Menu {
                title: "File";
                MenuItem {
                    title: "New";
                    shortcut: @keys(Control + N);
                    activated => { root.new-document(); }
                }
                MenuItem {
                    title: "Reset Demo";
                    activated => { root.reset-document(); }
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
                height: 50px;
                background: #f5f6f8;
                accessible-role: search;
                accessible-id: "document-search";
                accessible-label: "Find in document";

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
                        horizontal-stretch: 1;
                        accessible-id: "document-search-input";
                    }

                    Button {
                        text: "Clear";
                        clicked => { root.search-text = ""; }
                    }

                    Button {
                        text: "New";
                        clicked => { root.new-document(); }
                    }
                }
            }

            Rectangle {
                vertical-stretch: 1;
                background: #dfe3e8;
                accessible-role: main;
                accessible-id: "document-workspace";
                accessible-label: "Document editor";

                Rectangle {
                    x: 9%;
                    y: 24px;
                    width: 82%;
                    height: parent.height - 48px;
                    background: white;
                    border-width: 1px;
                    border-color: #b8bec6;

                    editor := TextEdit {
                        x: 54px;
                        y: 54px;
                        width: parent.width - 108px;
                        height: parent.height - 108px;
                        text <=> root.document-text;
                        wrap: word-wrap;
                        font-size: 16px * root.zoom-percent / 100;
                        accessible-id: "document-editor-text";
                        edited(text) => { root.document-edited(text); }
                    }
                }
            }

            Rectangle {
                height: 34px;
                background: #f5f6f8;
                accessible-role: content-info;
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

                    Button {
                        text: "−";
                        clicked => {
                            if root.zoom-percent > 50 {
                                root.zoom-percent -= 10;
                            }
                        }
                    }

                    Text {
                        text: root.zoom-percent + "%";
                        vertical-alignment: center;
                    }

                    Button {
                        text: "+";
                        clicked => {
                            if root.zoom-percent < 200 {
                                root.zoom-percent += 10;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn status_text(snapshot: &EditorSnapshot) -> SharedString {
    format!(
        "Authority {} · Revision {} · {} bytes",
        snapshot.authority_generation(),
        snapshot.revision(),
        snapshot.text().len()
    )
    .into()
}

fn publish_snapshot(ui: &DesktopShell, snapshot: &EditorSnapshot) {
    ui.set_document_text(snapshot.text().into());
    ui.set_status_text(status_text(snapshot));
}

fn main() -> Result<(), slint::PlatformError> {
    let app = Rc::new(RefCell::new(AppCore::new(MockDocumentEngine::default())));
    let initial = app
        .borrow_mut()
        .open_text_document(STARTER_DOCUMENT.to_owned())
        .expect("starter document must open in the bootstrap engine");

    let ui = DesktopShell::new()?;
    publish_snapshot(&ui, &initial);

    {
        let app = Rc::clone(&app);
        let weak_ui = ui.as_weak();
        ui.on_document_edited(move |text| {
            let result = app.borrow_mut().replace_document_text(text.as_str());
            let Some(ui) = weak_ui.upgrade() else {
                return;
            };
            match result {
                Ok(snapshot) => publish_snapshot(&ui, &snapshot),
                Err(error) => {
                    if let Ok(snapshot) = app.borrow().snapshot() {
                        publish_snapshot(&ui, &snapshot);
                    }
                    ui.set_status_text(format!("Edit rejected: {error:?}").into());
                }
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak_ui = ui.as_weak();
        ui.on_new_document(move || {
            let result = app.borrow_mut().open_text_document(String::new());
            if let Some(ui) = weak_ui.upgrade() {
                match result {
                    Ok(snapshot) => publish_snapshot(&ui, &snapshot),
                    Err(error) => ui.set_status_text(format!("New document failed: {error:?}").into()),
                }
            }
        });
    }

    {
        let app = Rc::clone(&app);
        let weak_ui = ui.as_weak();
        ui.on_reset_document(move || {
            let result = app
                .borrow_mut()
                .open_text_document(STARTER_DOCUMENT.to_owned());
            if let Some(ui) = weak_ui.upgrade() {
                match result {
                    Ok(snapshot) => publish_snapshot(&ui, &snapshot),
                    Err(error) => ui.set_status_text(format!("Reset failed: {error:?}").into()),
                }
            }
        });
    }

    ui.run()
}
