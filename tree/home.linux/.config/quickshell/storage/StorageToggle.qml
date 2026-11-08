import "../components"
import QtQuick

ToggleSwitch {
    activeFocusOnTab: true
    Accessible.role: Accessible.CheckBox
    Accessible.checked: checked
    Accessible.onPressAction: toggled()
    Keys.onSpacePressed: toggled()
    opacity: enabled ? 1 : 0.4
}
