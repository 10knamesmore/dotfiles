import Quickshell
import Quickshell.Hyprland._GlobalShortcuts
import "./screen-effects"

ShellRoot {
    GlobalShortcut {
        appid: "quickshell"
        name: "screenEffects"
        description: "Toggle screen effects panel"
        onPressed: panel.togglePanel()
    }

    ScreenEffectsPanel {
        id: panel
    }
}
