import "../theme"
import "Annotations.js" as Drawing
import QtQuick
import QtQuick.Controls
import QtQuick.Window

// 单个显示器的冻结原图、选区和标注。编辑坐标全部使用原图物理像素，
// 只有显示与鼠标输入换算成逻辑坐标；导出保持原图尺寸且不重新截屏。
Item {
    id: root

    required property var screenInfo
    readonly property bool active: CaptureService.activeScreen === screenInfo.name
    readonly property real displayScale: width / screenInfo.pixelWidth
    property rect selection: Qt.rect(0, 0, 0, 0)
    readonly property bool hasSelection: selection.width >= 2 && selection.height >= 2
    property string selectedWindowId: ""
    property string tool: "select"
    property string ink: "#f38ba8"
    property int strokeWidth: 4
    property var annotations: []
    property var redoStack: []
    property var draft: null
    property var pixelateSource: null
    property var sourcePixels: null
    property string pixelateImageUrl: ""
    property var pixelateGrab: null
    property int pixelateSerial: 0
    property bool preparingPixelation: false
    readonly property bool imageReady: artwork.imageReady
    readonly property string originalImageUrl: "file://" + screenInfo.image
    property string dragKind: ""
    property point dragStart: Qt.point(0, 0)
    property rect dragSelection: Qt.rect(0, 0, 0, 0)
    property point pointer: Qt.point(0, 0)
    property point textPosition: Qt.point(0, 0)
    property string sampledColor: "#000000"
    property string sampledRgb: "rgb(0,0,0)"
    property bool rgbFormat: false
    readonly property bool pickingColor: CaptureService.mode === "color" || tool === "eyedropper"
    readonly property var windowChoices: CaptureService.mode === "portal" ? (CaptureService.picker ? CaptureService.picker.windows : []) : []

    focus: active
    clip: true
    Accessible.role: Accessible.Pane
    Accessible.name: "屏幕捕获 · " + screenInfo.name
    Accessible.description: "拖动框选，Enter 复制，Ctrl+S 保存，Esc 取消"

    function reset() {
        selection = Qt.rect(0, 0, 0, 0);
        selectedWindowId = "";
        annotations = [];
        redoStack = [];
        draft = null;
        pixelateSource = null;
        tool = "select";
        textInput.visible = false;
    }

    function pixelPoint(x, y) {
        return Qt.point(Math.max(0, Math.min(screenInfo.pixelWidth - 1, x / displayScale)), Math.max(0, Math.min(screenInfo.pixelHeight - 1, y / displayScale)));
    }

    function setSelection(x1, y1, x2, y2) {
        const b = Drawing.rectangle(x1, y1, x2, y2);
        selection = Qt.rect(Math.round(b.x), Math.round(b.y), Math.round(b.width), Math.round(b.height));
    }

    function selectOutput() {
        selectedWindowId = "";
        selection = Qt.rect(0, 0, screenInfo.pixelWidth, screenInfo.pixelHeight);
    }

    function selectWindow(window) {
        const scale = screenInfo.scale;
        const x1 = Math.max(0, (window.x - screenInfo.x) * scale);
        const y1 = Math.max(0, (window.y - screenInfo.y) * scale);
        const x2 = Math.min(screenInfo.pixelWidth, (window.x + window.width - screenInfo.x) * scale);
        const y2 = Math.min(screenInfo.pixelHeight, (window.y + window.height - screenInfo.y) * scale);
        if (x2 > x1 && y2 > y1)
            setSelection(x1, y1, x2, y2);
    }

    function choosePortalWindow(index) {
        const window = windowChoices[index];
        if (!window)
            return;
        selectedWindowId = window.id;
        selection = Qt.rect(0, 0, 0, 0);
        const desktopWindow = CaptureService.snapshot.windows.find(w => parseInt(w.address, 16) === parseInt(window.address, 16));
        if (desktopWindow && desktopWindow.monitor === screenInfo.name && desktopWindow.workspaceId === screenInfo.activeWorkspace)
            selectWindow(desktopWindow);
    }

    function windowAt(point) {
        const x = screenInfo.x + point.x / screenInfo.scale;
        const y = screenInfo.y + point.y / screenInfo.scale;
        const windows = CaptureService.snapshot.windows.filter(w => w.monitor === screenInfo.name && w.workspaceId === screenInfo.activeWorkspace && x >= w.x && y >= w.y && x < w.x + w.width && y < w.y + w.height);
        return windows.find(w => w.focused) || windows[0];
    }

    function handleAt(point) {
        if (!hasSelection)
            return "new";
        const s = selection, distance = 9 / displayScale;
        if (point.x < s.x - distance || point.x > s.x + s.width + distance || point.y < s.y - distance || point.y > s.y + s.height + distance)
            return "new";
        const horizontal = Math.abs(point.x - s.x) < distance ? "l" : Math.abs(point.x - s.x - s.width) < distance ? "r" : "";
        const vertical = Math.abs(point.y - s.y) < distance ? "t" : Math.abs(point.y - s.y - s.height) < distance ? "b" : "";
        return horizontal + vertical || "move";
    }

    function updateSelection(point) {
        const s = dragSelection, dx = point.x - dragStart.x, dy = point.y - dragStart.y;
        if (dragKind === "new") {
            setSelection(dragStart.x, dragStart.y, point.x, point.y);
        } else if (dragKind === "move") {
            selection = Qt.rect(Math.round(Math.max(0, Math.min(screenInfo.pixelWidth - s.width, s.x + dx))), Math.round(Math.max(0, Math.min(screenInfo.pixelHeight - s.height, s.y + dy))), s.width, s.height);
        } else {
            const left = dragKind.indexOf("l") >= 0 ? point.x : s.x;
            const right = dragKind.indexOf("r") >= 0 ? point.x : s.x + s.width;
            const top = dragKind.indexOf("t") >= 0 ? point.y : s.y;
            const bottom = dragKind.indexOf("b") >= 0 ? point.y : s.y + s.height;
            setSelection(left, top, right, bottom);
        }
    }

    function appendAnnotation(annotation) {
        annotations = annotations.concat([annotation]);
        redoStack = [];
    }

    function updateDraft(point) {
        const d = draft;
        d.x2 = point.x;
        d.y2 = point.y;
        if (d.type === "pixelate")
            d.cells = Drawing.pixelateCells(pixelateSource, d);
        if (d.type === "brush")
            d.points.push({
                x: point.x,
                y: point.y
            });
    }

    function commitText() {
        if (!textInput.visible)
            return;
        if (textInput.text.trim()) {
            appendAnnotation({
                type: "text",
                x1: textPosition.x,
                y1: textPosition.y,
                x2: textPosition.x,
                y2: textPosition.y,
                color: ink,
                width: strokeWidth * screenInfo.scale,
                text: textInput.text,
                fontSize: 22 * screenInfo.scale
            });
        }
        textInput.visible = false;
        root.forceActiveFocus();
    }

    function undo() {
        if (!annotations.length)
            return;
        redoStack = redoStack.concat([annotations[annotations.length - 1]]);
        annotations = annotations.slice(0, -1);
        if (tool === "pixelate")
            preparePixelation();
    }

    function redo() {
        if (!redoStack.length)
            return;
        annotations = annotations.concat([redoStack[redoStack.length - 1]]);
        redoStack = redoStack.slice(0, -1);
        if (tool === "pixelate")
            preparePixelation();
    }

    function sample(point) {
        if (!imageReady || !sourcePixels)
            return false;
        // 从原始 PNG 读像素，不读取被 Qt DPR 放大的显示缓冲。
        const index = (Math.floor(point.y) * sourcePixels.width + Math.floor(point.x)) * 4;
        const data = sourcePixels.data;
        const pixel = [data[index], data[index + 1], data[index + 2]];
        sampledColor = Drawing.hexColor(pixel);
        sampledRgb = "rgb(" + pixel[0] + "," + pixel[1] + "," + pixel[2] + ")";
        lensCanvas.requestPaint();
        return true;
    }

    // Qt 会把 targetSize 再乘窗口有效 DPR。Wayland 分数缩放下 Screen 的 DPR
    // 可能取整，必须读实际窗口；saveToFile 则不受虚拟 QML baseUrl 影响。
    function grabArtwork(completed) {
        const pixelRatio = root.Window.window.devicePixelRatio;
        const started = artwork.grabToImage(completed, Qt.size(Math.round(screenInfo.pixelWidth / pixelRatio), Math.round(screenInfo.pixelHeight / pixelRatio)));
        if (!started) {
            CaptureService.errorText = "无法读取标注画布";
            console.error("[capture] artwork grab failed");
            completed(null);
        }
    }

    function renderArtwork(path, completed) {
        grabArtwork(result => {
            const saved = result !== null && result.saveToFile(path);
            if (result && !saved) {
                CaptureService.errorText = "无法生成标注图片";
                console.error("[capture] artwork save failed");
            }
            completed(saved);
        });
    }

    function preparePixelation() {
        const serial = ++pixelateSerial;
        preparingPixelation = true;
        grabArtwork(result => {
            if (serial !== pixelateSerial || tool !== "pixelate")
                return;
            if (!result) {
                preparingPixelation = false;
                tool = "select";
                return;
            }
            if (pixelateImageUrl)
                pixelReader.unloadImage(pixelateImageUrl);
            // 持有内存截图到像素读取完成，不在每一笔结束时编码、写入再解码整屏 PNG。
            pixelateGrab = result;
            pixelateImageUrl = result.url.toString();
            pixelReader.loadImage(pixelateImageUrl);
            acceptPixelationImage();
        });
    }

    function acceptPixelationImage() {
        if (!preparingPixelation || !pixelReader.isImageLoaded(pixelateImageUrl))
            return;
        pixelateSource = pixelReader.getContext("2d").createImageData(pixelateImageUrl);
        preparingPixelation = false;
    }

    function loadSourcePixels() {
        if (!pixelReader.available || sourcePixels || !pickingColor)
            return;
        pixelReader.loadImage(originalImageUrl);
        acceptSourcePixels();
    }

    function acceptSourcePixels() {
        if (sourcePixels || !pixelReader.isImageLoaded(originalImageUrl))
            return;
        sourcePixels = pixelReader.getContext("2d").createImageData(originalImageUrl);
        sample(pointer);
    }

    onPickingColorChanged: loadSourcePixels()

    onToolChanged: {
        if (tool === "pixelate") {
            preparePixelation();
        } else {
            ++pixelateSerial;
            preparingPixelation = false;
            pixelateSource = null;
            if (pixelateImageUrl)
                pixelReader.unloadImage(pixelateImageUrl);
            pixelateImageUrl = "";
            pixelateGrab = null;
        }
    }

    function finish(action) {
        if (CaptureService.exporting || preparingPixelation || !imageReady)
            return;
        if (CaptureService.mode === "portal") {
            if (CaptureService.target === "window" && selectedWindowId) {
                CaptureService.confirmPicker({
                    type: "window",
                    id: selectedWindowId
                });
            } else if (hasSelection) {
                const scale = screenInfo.scale;
                if (CaptureService.target === "output") {
                    CaptureService.confirmPicker({
                        type: "output",
                        output: screenInfo.name
                    });
                } else {
                    const x = Math.round(selection.x / scale), y = Math.round(selection.y / scale);
                    const right = Math.min(Math.floor(screenInfo.width), Math.round((selection.x + selection.width) / scale));
                    const bottom = Math.min(Math.floor(screenInfo.height), Math.round((selection.y + selection.height) / scale));
                    CaptureService.confirmPicker({
                        type: "region",
                        output: screenInfo.name,
                        x: x,
                        y: y,
                        width: right - x,
                        height: bottom - y
                    });
                }
            }
        } else if (CaptureService.mode === "screenshot" && hasSelection) {
            commitText();
            const path = screenInfo.image + ".annotated.bmp";
            CaptureService.exporting = true;
            renderArtwork(path, saved => {
                if (saved)
                    CaptureService.exportImage(path, {
                        x: selection.x,
                        y: selection.y,
                        width: selection.width,
                        height: selection.height
                    }, action);
                else
                    CaptureService.exporting = false;
            });
        }
    }

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Escape) {
            if (tool === "eyedropper" && CaptureService.mode === "screenshot")
                tool = "select";
            else
                CaptureService.cancel();
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            finish("copy");
        } else if (event.modifiers & Qt.ControlModifier && event.key === Qt.Key_S) {
            finish("save");
        } else if (event.modifiers & Qt.ControlModifier && event.key === Qt.Key_Z) {
            if (event.modifiers & Qt.ShiftModifier)
                redo();
            else
                undo();
        } else if (event.modifiers & Qt.ControlModifier && event.key === Qt.Key_Y) {
            redo();
        } else {
            event.accepted = false;
            return;
        }
        event.accepted = true;
    }

    CaptureArtwork {
        id: artwork
        anchors.fill: parent
        source: root.originalImageUrl
        annotations: root.annotations
        draft: root.draft
        coordinateScale: root.displayScale
    }

    // 只在取色或马赛克需要时读取像素，不创建整屏显示缓冲，也不参与画面导出。
    Canvas {
        id: pixelReader
        width: 1
        height: 1
        opacity: 0
        contextType: "2d"
        onAvailableChanged: root.loadSourcePixels()
        onImageLoaded: {
            root.acceptSourcePixels();
            root.acceptPixelationImage();
        }
    }

    Item {
        anchors.fill: parent
        visible: !root.pickingColor
        readonly property real sx: root.active ? root.selection.x * root.displayScale : 0
        readonly property real sy: root.active ? root.selection.y * root.displayScale : 0
        readonly property real sw: root.active ? root.selection.width * root.displayScale : 0
        readonly property real sh: root.active ? root.selection.height * root.displayScale : 0
        Rectangle {
            x: 0
            y: 0
            width: parent.width
            height: parent.sy
            color: "#80000000"
        }
        Rectangle {
            x: 0
            y: parent.sy
            width: parent.sx
            height: parent.sh
            color: "#80000000"
        }
        Rectangle {
            x: parent.sx + parent.sw
            y: parent.sy
            width: parent.width - x
            height: parent.sh
            color: "#80000000"
        }
        Rectangle {
            x: 0
            y: parent.sy + parent.sh
            width: parent.width
            height: parent.height - y
            color: "#80000000"
        }
    }

    Rectangle {
        x: root.selection.x * root.displayScale
        y: root.selection.y * root.displayScale
        width: root.selection.width * root.displayScale
        height: root.selection.height * root.displayScale
        visible: root.hasSelection && root.active && !root.pickingColor
        color: "transparent"
        border.color: Colors.blue
        border.width: 2
        Repeater {
            model: [[0, 0], [0.5, 0], [1, 0], [0, 0.5], [1, 0.5], [0, 1], [0.5, 1], [1, 1]]
            Rectangle {
                required property var modelData
                x: modelData[0] * parent.width - 4
                y: modelData[1] * parent.height - 4
                width: 8
                height: 8
                radius: 2
                color: Colors.blue
                visible: root.tool === "select" && CaptureService.target === "region"
            }
        }
    }

    MouseArea {
        id: pointerArea
        anchors.fill: parent
        hoverEnabled: true
        // 选区只依赖尺寸，可在底图异步解码时开始；录制和导出期间仍拦截桌面点击。
        cursorShape: CaptureService.mode.startsWith("record-") ? Qt.ArrowCursor : root.tool === "text" ? Qt.IBeamCursor : Qt.CrossCursor
        onPressed: mouse => {
            if (CaptureService.exporting || root.preparingPixelation || CaptureService.mode.startsWith("record-"))
                return;
            CaptureService.activeScreen = root.screenInfo.name;
            root.forceActiveFocus();
            root.commitText();
            const p = root.pixelPoint(mouse.x, mouse.y);
            root.pointer = p;
            if (root.pickingColor) {
                if (!root.sample(p))
                    return;
                if (CaptureService.mode === "color")
                    CaptureService.copyColor(root.rgbFormat ? root.sampledRgb : root.sampledColor);
                else {
                    root.ink = root.sampledColor;
                    root.tool = "select";
                }
                return;
            }
            if (CaptureService.target === "window" && (CaptureService.mode === "portal" || !root.hasSelection)) {
                const window = root.windowAt(p);
                if (window) {
                    if (CaptureService.mode === "portal") {
                        const index = root.windowChoices.findIndex(w => parseInt(w.address, 16) === parseInt(window.address, 16));
                        root.choosePortalWindow(index);
                    } else {
                        root.selectWindow(window);
                    }
                }
                return;
            }
            if (CaptureService.target === "output" && !root.hasSelection) {
                root.selectOutput();
                return;
            }
            if (root.tool === "select" || CaptureService.mode === "portal") {
                if (CaptureService.target !== "region")
                    return;
                root.dragKind = root.handleAt(p);
                root.dragStart = p;
                root.dragSelection = root.selection;
                if (root.dragKind === "new") {
                    root.annotations = [];
                    root.redoStack = [];
                    root.setSelection(p.x, p.y, p.x, p.y);
                }
            } else if (root.hasSelection) {
                if (p.x < root.selection.x || p.y < root.selection.y || p.x > root.selection.x + root.selection.width || p.y > root.selection.y + root.selection.height)
                    return;
                if (root.tool === "text") {
                    root.textPosition = p;
                    textInput.text = "";
                    textInput.visible = true;
                    textInput.forceActiveFocus();
                } else {
                    root.draft = {
                        type: root.tool,
                        cells: [],
                        x1: p.x,
                        y1: p.y,
                        x2: p.x,
                        y2: p.y,
                        color: root.ink,
                        width: root.strokeWidth * root.screenInfo.scale,
                        points: [
                            {
                                x: p.x,
                                y: p.y
                            }
                        ]
                    };
                }
            }
        }
        onPositionChanged: mouse => {
            if (CaptureService.exporting || CaptureService.mode.startsWith("record-"))
                return;
            const p = root.pixelPoint(mouse.x, mouse.y);
            root.pointer = p;
            if (root.pickingColor)
                root.sample(p);
            if (!pressed)
                return;
            if (root.dragKind) {
                root.updateSelection(p);
            } else if (root.draft) {
                root.updateDraft(p);
                artwork.repaintDraft();
            }
        }
        onReleased: mouse => {
            // 移动事件可能被合并，必须用释放事件的最终坐标提交，不能停在上一帧。
            const p = root.pixelPoint(mouse.x, mouse.y);
            if (root.dragKind)
                root.updateSelection(p);
            root.dragKind = "";
            if (root.draft) {
                root.updateDraft(p);
                const pixelated = root.draft.type === "pixelate";
                root.appendAnnotation(root.draft);
                root.draft = null;
                if (pixelated)
                    root.preparePixelation();
            }
        }
        onCanceled: {
            root.dragKind = "";
            root.draft = null;
        }
    }

    // 只导出 artwork 中的画面与标注，不包含输入框、选区边框和工具条。
    TextArea {
        id: textInput
        visible: false
        x: root.textPosition.x * root.displayScale
        y: root.textPosition.y * root.displayScale
        width: Math.max(100, Math.min(480, root.width - x - 10))
        height: Math.max(42, implicitHeight)
        color: root.ink
        font.family: "sans-serif"
        font.pixelSize: 22
        padding: 0
        wrapMode: TextEdit.NoWrap
        selectByMouse: true
        placeholderText: "输入标注，Enter 确认"
        Accessible.name: "标注文字"
        background: Rectangle {
            color: "#cc181825"
            border.color: Colors.blue
            radius: 3
        }
        Keys.onPressed: event => {
            if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && !(event.modifiers & Qt.ShiftModifier)) {
                root.commitText();
                event.accepted = true;
            } else if (event.key === Qt.Key_Escape) {
                visible = false;
                root.forceActiveFocus();
                event.accepted = true;
            }
        }
    }

    CaptureControls {
        id: controls
        surface: root
        anchors.top: parent.top
        anchors.topMargin: 22
        anchors.horizontalCenter: parent.horizontalCenter
        visible: root.active
    }

    CaptureTools {
        surface: root
        enabled: root.imageReady
        visible: root.active && root.hasSelection && CaptureService.mode === "screenshot" && !root.pickingColor
        x: Math.max(12, Math.min(root.width - width - 12, root.selection.x * root.displayScale))
        y: {
            const below = (root.selection.y + root.selection.height) * root.displayScale + 14;
            return below + height < root.height - 12 ? below : Math.max(controls.y + controls.height + 12, root.selection.y * root.displayScale - height - 14);
        }
    }

    Rectangle {
        id: lens
        visible: root.pickingColor && pointerArea.containsMouse && root.imageReady
        x: Math.min(root.width - width - 12, root.pointer.x * root.displayScale + 26)
        y: Math.min(root.height - height - 12, root.pointer.y * root.displayScale + 26)
        width: 154
        height: 194
        radius: 10
        color: Colors.base
        border.color: Colors.surface2
        Canvas {
            id: lensCanvas
            x: 11
            y: 11
            width: 132
            height: 132
            onPaint: {
                if (!root.imageReady || !root.sourcePixels)
                    return;
                const ctx = getContext("2d");
                ctx.reset();
                ctx.imageSmoothingEnabled = false;
                const x = Math.max(0, Math.min(root.screenInfo.pixelWidth - 11, Math.floor(root.pointer.x) - 5));
                const y = Math.max(0, Math.min(root.screenInfo.pixelHeight - 11, Math.floor(root.pointer.y) - 5));
                ctx.drawImage(root.sourcePixels, x, y, 11, 11, 0, 0, 132, 132);
                ctx.strokeStyle = "white";
                ctx.lineWidth = 1;
                ctx.strokeRect((Math.floor(root.pointer.x) - x) * 12, (Math.floor(root.pointer.y) - y) * 12, 12, 12);
            }
        }
        Rectangle {
            x: 11
            y: 157
            width: 22
            height: 22
            color: root.sampledColor
            radius: 3
            border.color: Colors.text
        }
        Text {
            x: 41
            y: 157
            text: root.sampledColor
            color: Colors.text
            font.pixelSize: 15
        }
    }

    onActiveChanged: {
        if (active)
            forceActiveFocus();
    }
    Connections {
        target: CaptureService
        function onResetEditor() {
            root.reset();
        }
    }
}
