import Quickshell
import Quickshell.Hyprland._GlobalShortcuts

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
