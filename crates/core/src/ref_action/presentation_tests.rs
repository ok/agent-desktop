use super::arrival_shape;
use crate::{Action, CursorShape};

#[test]
fn pressable_controls_show_the_pointer_however_they_are_reached() {
    for role in [
        "button",
        "link",
        "checkbox",
        "tab",
        "menuitem",
        "switch",
        "disclosure",
    ] {
        for action in [
            Action::Click,
            Action::ScrollTo,
            Action::SetFocus,
            Action::Toggle,
        ] {
            assert_eq!(
                arrival_shape(role, &action),
                CursorShape::Pointer,
                "{role} {action:?}"
            );
        }
    }
}

#[test]
fn text_entry_shows_the_text_caret_however_it_is_reached() {
    for role in ["textfield", "combobox", "datefield"] {
        for action in [
            Action::Click,
            Action::DoubleClick,
            Action::SetFocus,
            Action::ScrollTo,
            Action::SetValue("x".into()),
        ] {
            assert_eq!(
                arrival_shape(role, &action),
                CursorShape::Text,
                "{role} {action:?}"
            );
        }
    }
}

#[test]
fn other_elements_show_the_pointer_only_when_clicked() {
    for role in ["group", "cell", "statictext", "unknown", "row"] {
        assert_eq!(arrival_shape(role, &Action::Click), CursorShape::Pointer);
        assert_eq!(
            arrival_shape(role, &Action::RightClick),
            CursorShape::Pointer
        );
        assert_eq!(arrival_shape(role, &Action::ScrollTo), CursorShape::Arrow);
        assert_eq!(
            arrival_shape(role, &Action::SetValue("x".into())),
            CursorShape::Arrow,
            "{role}"
        );
    }
}
