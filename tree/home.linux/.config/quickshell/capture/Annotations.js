/** 原分辨率画布的标注绘制；输入坐标、笔宽和字号均为截图物理像素。 */

/** @param {number} x1 @param {number} y1 @param {number} x2 @param {number} y2 @returns {{x:number,y:number,width:number,height:number}} */
function rectangle(x1, y1, x2, y2) {
    return {x: Math.min(x1, x2), y: Math.min(y1, y2), width: Math.abs(x2 - x1), height: Math.abs(y2 - y1)};
}

/**
 * @typedef {Object} Annotation
 * @property {string} type
 * @property {string} color
 * @property {number} width
 * @property {number} x1
 * @property {number} y1
 * @property {number} x2
 * @property {number} y2
 * @property {Array<{x:number,y:number}>} [points]
 * @property {string} [text]
 * @property {number} [fontSize]
 * @property {Array<{x:number,y:number,width:number,height:number,color:string}>} [cells]
 */

/**
 * 拖动图形的物理像素包围框，包含笔宽及箭头尖；文字不经过拖动预览。
 * @param {Annotation} annotation
 * @returns {{x:number,y:number,width:number,height:number}}
 */
function gestureBounds(annotation) {
    const a = annotation;
    let left = Math.min(a.x1, a.x2), right = Math.max(a.x1, a.x2);
    let top = Math.min(a.y1, a.y2), bottom = Math.max(a.y1, a.y2);
    if (a.type === "brush") {
        for (const point of a.points) {
            left = Math.min(left, point.x);
            right = Math.max(right, point.x);
            top = Math.min(top, point.y);
            bottom = Math.max(bottom, point.y);
        }
    }
    const padding = (a.type === "arrow" ? Math.max(a.width * 4, 12) : a.width / 2) + 2;
    return {x: left - padding, y: top - padding, width: right - left + padding * 2, height: bottom - top + padding * 2};
}

/** @param {Object} context Qt Canvas 2D context @param {Annotation} annotation */
function draw(context, annotation) {
    const a = annotation;
    const box = rectangle(a.x1, a.y1, a.x2, a.y2);
    context.save();
    context.strokeStyle = a.color;
    context.fillStyle = a.color;
    context.lineWidth = a.width;
    context.lineCap = "round";
    context.lineJoin = "round";
    switch (a.type) {
    case "arrow": {
        const angle = Math.atan2(a.y2 - a.y1, a.x2 - a.x1);
        const head = Math.max(a.width * 4, 12);
        context.beginPath();
        context.moveTo(a.x1, a.y1);
        context.lineTo(a.x2, a.y2);
        context.moveTo(a.x2 - head * Math.cos(angle - Math.PI / 6), a.y2 - head * Math.sin(angle - Math.PI / 6));
        context.lineTo(a.x2, a.y2);
        context.lineTo(a.x2 - head * Math.cos(angle + Math.PI / 6), a.y2 - head * Math.sin(angle + Math.PI / 6));
        context.stroke();
        break;
    }
    case "rectangle":
        context.strokeRect(box.x, box.y, box.width, box.height);
        break;
    case "redact":
        context.fillRect(box.x, box.y, box.width, box.height);
        break;
    case "brush":
        context.beginPath();
        context.moveTo(a.points[0].x, a.points[0].y);
        for (let i = 1; i < a.points.length; i++)
            context.lineTo(a.points[i].x, a.points[i].y);
        if (a.points.length === 1)
            context.lineTo(a.points[0].x + 0.1, a.points[0].y);
        context.stroke();
        break;
    case "text":
        context.font = a.fontSize + "px sans-serif";
        context.textBaseline = "top";
        const lines = a.text.split("\n");
        for (let i = 0; i < lines.length; i++)
            context.fillText(lines[i], a.x1, a.y1 + i * a.fontSize * 1.3);
        break;
    case "pixelate":
        for (const cell of a.cells) {
            context.fillStyle = cell.color;
            context.fillRect(cell.x, cell.y, cell.width, cell.height);
        }
        break;
    }
    context.restore();
}

/**
 * 用手势前准备好的合成底图生成固定色块。不能在 paint 中边写边读画布，
 * 否则异步重绘和导出可能读到不同帧，导致最终图片与预览不一致。
 * @param {{data:ArrayLike<number>,width:number,height:number}} source
 * @param {Annotation} annotation
 * @returns {Array<{x:number,y:number,width:number,height:number,color:string}>}
 */
function pixelateCells(source, annotation) {
    const box = rectangle(annotation.x1, annotation.y1, annotation.x2, annotation.y2);
    const x = Math.floor(box.x), y = Math.floor(box.y);
    const w = Math.floor(box.width), h = Math.floor(box.height);
    const block = Math.max(8, Math.round(annotation.width * 4));
    const cells = [];
    const pixels = source.data;
    for (let dy = 0; dy < h; dy += block) {
        for (let dx = 0; dx < w; dx += block) {
            const sampleX = x + Math.min(w - 1, dx + Math.floor(block / 2));
            const sampleY = y + Math.min(h - 1, dy + Math.floor(block / 2));
            const i = (sampleY * source.width + sampleX) * 4;
            cells.push({x: x + dx, y: y + dy, width: Math.min(block, w - dx), height: Math.min(block, h - dy),
                color: "rgb(" + pixels[i] + "," + pixels[i + 1] + "," + pixels[i + 2] + ")"});
        }
    }
    return cells;
}

/** @param {ArrayLike<number>} pixel RGBA字节 @returns {string} */
function hexColor(pixel) {
    return "#" + [pixel[0], pixel[1], pixel[2]].map(value => value.toString(16).padStart(2, "0")).join("");
}
