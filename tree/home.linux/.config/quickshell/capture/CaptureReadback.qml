import QtQuick

// 只回读指定的物理像素区域。Qt 按窗口 DPR 放大 grabToImage 的整数尺寸，
// 因而向外补齐不足一个逻辑像素的边缘；导出后端移除这部分补齐，不缩放原图。
ShaderEffectSource {
    id: root

    required property Item capturedItem
    required property rect region
    required property real coordinateScale
    required property real windowPixelRatio

    width: Math.ceil(region.width / windowPixelRatio)
    height: Math.ceil(region.height / windowPixelRatio)
    opacity: 0
    smooth: false
    live: false
    sourceItem: capturedItem
    textureSize: Qt.size(Math.round(width * windowPixelRatio), Math.round(height * windowPixelRatio))
    sourceRect: Qt.rect(region.x * coordinateScale, region.y * coordinateScale, textureSize.width * coordinateScale, textureSize.height * coordinateScale)

    function read(completed) {
        const started = grabToImage(result => {
            completed(result);
            root.destroy();
        }, Qt.size(width, height));
        if (!started) {
            completed(null);
            destroy();
        }
    }
}
