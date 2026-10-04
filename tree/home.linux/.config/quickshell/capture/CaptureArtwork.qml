import "Annotations.js" as Drawing
import QtQuick

// 冻结底图只上传一次；已提交的标注仅在编辑历史变化时重绘。
// 拖动只更新图形包围框内的透明画布，三个图层一起用于原分辨率导出。
Item {
    id: root

    required property url source
    required property var annotations
    required property var draft
    required property real coordinateScale // 原图物理像素到此 Item 逻辑坐标的比例。
    readonly property bool imageReady: background.status === Image.Ready

    function repaintDraft() {
        draftCanvas.redraw();
    }

    Image {
        id: background
        anchors.fill: parent
        source: root.source
        asynchronous: true
        cache: false
        smooth: false
    }

    Canvas {
        id: committedCanvas
        anchors.fill: parent
        renderTarget: Canvas.Image
        smooth: false
        property var annotations: root.annotations
        onAnnotationsChanged: requestPaint()
        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            ctx.clearRect(0, 0, width, height);
            ctx.scale(root.coordinateScale, root.coordinateScale);
            for (const annotation of annotations)
                Drawing.draw(ctx, annotation);
        }
    }

    Canvas {
        id: draftCanvas
        visible: annotation !== null
        renderTarget: Canvas.Image
        smooth: false
        property var annotation: root.draft
        onAnnotationChanged: redraw()

        function redraw() {
            if (!annotation)
                return;
            const bounds = Drawing.gestureBounds(annotation);
            const scale = root.coordinateScale;
            // 向外对齐到 32 个逻辑像素，避免拖动时每移动一像素就重分配纹理。
            x = Math.floor(bounds.x * scale / 32) * 32;
            y = Math.floor(bounds.y * scale / 32) * 32;
            width = Math.ceil((bounds.x + bounds.width) * scale / 32) * 32 - x;
            height = Math.ceil((bounds.y + bounds.height) * scale / 32) * 32 - y;
            requestPaint();
        }

        onPaint: {
            const ctx = getContext("2d");
            ctx.reset();
            ctx.clearRect(0, 0, width, height);
            if (!annotation)
                return;
            ctx.translate(-x, -y);
            ctx.scale(root.coordinateScale, root.coordinateScale);
            Drawing.draw(ctx, annotation);
        }
    }
}
