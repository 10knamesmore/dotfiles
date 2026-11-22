use std::collections::BTreeMap;

use serde_json::{Value, json};

// Mermaid 12.3.0 resolved theme variables, sampled in merman's generated/theme_variables.json.
// Theme order: dark, forest, neutral, base, default.
const PRIMARY_TEXT: [&str; 5] = ["#e0dfdf", "#321b67", "#111111", "#333333", "#131300"];

const COLORS: &[(&str, [&str; 5])] = &[
    (
        "background",
        ["#333333", "#ffffff", "#ffffff", "#f4f4f4", "#ffffff"],
    ),
    (
        "textColor",
        ["#cccccc", "#000000", "#000000", "#333333", "#333333"],
    ),
    (
        "primaryColor",
        ["#1f2020", "#cde498", "#eeeeee", "#fff4dd", "#ececff"],
    ),
    (
        "primaryBorderColor",
        ["#cccccc", "#abb594", "#d5d5d5", "#eedebb", "#c7c7f1"],
    ),
    ("primaryTextColor", PRIMARY_TEXT),
    (
        "lineColor",
        ["#d3d3d3", "#000000", "#666666", "#0b0b0b", "#333333"],
    ),
    (
        "actorBkg",
        ["#1f2020", "#cde498", "#eeeeee", "#fff4dd", "#ececff"],
    ),
    (
        "actorBorder",
        ["#cccccc", "#a6cf47", "#d4d4d4", "#eedebb", "#9370db"],
    ),
    (
        "actorTextColor",
        ["#d3d3d3", "#000000", "#333333", "#333333", "#000000"],
    ),
    (
        "noteBkgColor",
        ["#474949", "#fff5ad", "#666666", "#fff5ad", "#fff5ad"],
    ),
    (
        "noteBorderColor",
        ["#2f2f2f", "#6eaa49", "#999999", "#e4db95", "#aaaa33"],
    ),
    (
        "noteTextColor",
        ["#b8b6b6", "#000000", "#ffffff", "#333333", "#000000"],
    ),
    (
        "signalColor",
        ["#d3d3d3", "#333333", "#333333", "#333333", "#333333"],
    ),
    (
        "signalTextColor",
        ["#d3d3d3", "#333333", "#333333", "#333333", "#333333"],
    ),
    (
        "pieTitleTextColor",
        ["#d3d3d3", "#000000", "#333333", "#333333", "#000000"],
    ),
    (
        "pieLegendTextColor",
        ["#d3d3d3", "#000000", "#333333", "#333333", "#000000"],
    ),
    (
        "pieSectionTextColor",
        ["#cccccc", "#000000", "#000000", "#333333", "#333333"],
    ),
    (
        "commitLabelColor",
        ["#b8b6b6", "#32004d", "#030303", "#0b2200", "#000021"],
    ),
    (
        "commitLabelBackground",
        ["#474949", "#cdffb2", "#fcfcfc", "#f4ddff", "#ffffde"],
    ),
];

const PIE: [&str; 5] = [
    "#0b0000 #4d1037 #3f5258 #4f2f1b #6e0a0a #3b0048 #995a01 #154706 #161722 #00296f #01629c",
    "#cde498 #cdffb2 #e1efc0 #8cb42f #6aff19 #33b42f #70d990 #d99070 #98cde4 #1a6330 #63301a #1a4d63",
    "#f4f4f4 #555555 #bbbbbb #777777 #999999 #dddddd #ffffff #dddddd #bbbbbb #999999 #777777 #555555",
    "#fff4dd #f4ddff #f7f9ff #ffe3aa #e3aaff #c4d7ff #c6ffaa #ffaac6 #ddfff4 #a3ff77 #ff77a3 #aaffe4",
    "#ececff #ffffde #b5ff20 #b9b9ff #ffff45 #d7ff86 #ff86ff #20ffff #ff2020 #ff20ff #20ff90 #ff5353",
];

const GIT: [&str; 5] = [
    "#797d7d #a12273 #6a8993 #9b5c35 #cc1212 #65007b #cc7801 #31a50e",
    "#9bc834 #7aff33 #b0d45b #c8ab34 #c86134 #c83452 #34c861 #349bc8",
    "#b4b4b4 #555555 #bbbbbb #777777 #999999 #dddddd #ffffff #dddddd",
    "#ffcb5e #cb5eff #77a3ff #ff7a5e #ff5e92 #ff5ee3 #92ff5e #5effcb",
    "#0000ec #dede00 #9dec00 #0076ec #00ecec #00ec76 #ec00ec #ec0000",
];

const GIT_LABELS: [&str; 5] = [
    "#2c2c2c #d3d3d3 #d3d3d3 #2c2c2c #d3d3d3 #d3d3d3 #d3d3d3 #d3d3d3",
    "#ffffff #000000 #000000 #ffffff #000000 #000000 #000000 #000000",
    "#333333 #ffffff #333333 #ffffff #333333 #333333 #333333 #333333",
    "#333333 #333333 #333333 #333333 #333333 #333333 #333333 #333333",
    "#ffffff #000000 #000000 #ffffff #000000 #000000 #000000 #000000",
];

const GIT_HIGHLIGHTS: [&str; 5] = [
    "#868282 #5edd8c #95766c #64a3ca #34eded #9aff84 #3387fe #ce5af1",
    "#6437cb #8500cd #4f2ba4 #3754cb #379ecb #37cbad #cb379e #cb6437",
    "#4b4b4b #aaaaaa #444444 #888888 #666666 #222222 #000000 #222222",
    "#0034a2 #34a200 #885c00 #0085a2 #00a26d #00a21d #6d00a2 #a20034",
    "#131300 #0000a1 #310093 #934900 #930000 #930049 #009300 #009393",
];

const RADAR_CURVES: [&str; 5] = [
    "#1f2020 #0b0000 #4d1037 #3f5258 #4f2f1b #6e0a0a #3b0048 #995a01 #154706 #161722 #00296f #01629c",
    "#b9d970 #acff7f #cde498 #84d970 #70d990 #70d9c5 #70b9d9 #7084d9 #c570d9 #d97084 #d99070 #d9c570",
    "#555555 #f4f4f4 #555555 #bbbbbb #777777 #999999 #dddddd #ffffff #dddddd #bbbbbb #999999 #777777",
    "#ffcb5d #cb5dff #77a3ff #e3ff5d #92ff5d #5dff7a #5dffcb #5de3ff #9680ff #ff5de3 #ff5d92 #ff7a5d",
    "#8686ff #ffff78 #d7ff86 #c286ff #ff86ff #ff86c2 #ff8686 #ffc286 #c2ff86 #86ffc2 #86ffff #86c2ff",
];

const RADAR_TITLE: [&str; 5] = ["#f9fffe", "#333333", "#333333", "#090600", "#333333"];

const PACKET_FOREGROUND: [&str; 5] = [
    PRIMARY_TEXT[0],
    PRIMARY_TEXT[1],
    "#000000",
    "#000000",
    "#000000",
];

const XY: [&str; 5] = [
    "#3498db,#2ecc71,#e74c3c,#f1c40f,#bdc3c7,#ffffff,#34495e,#9b59b6,#1abc9c,#e67e22",
    "#cde498,#ff6b6b,#a0d2db,#d7bde2,#f0f0f0,#ffc3a0,#7fd8be,#ff9a8b,#faf3e0,#fff176",
    "#eee,#6bb8e4,#8acb88,#c7acd6,#e8dcc2,#ffb2a8,#fff380,#7e8d91,#ffd8b1,#faf3e0",
    "#fff4dd,#ffd8b1,#ffa07a,#eceff1,#d6dbdf,#c3e0a8,#ffb6a4,#ffd74d,#738fa7,#fffff0",
    "#ececff,#8493a6,#ffc3a0,#dcdde1,#b8e994,#d1a36f,#c3cde6,#ffb6c1,#496078,#f8f3e3",
];

pub(super) fn variables(name: &str, diagram: &str) -> Option<BTreeMap<String, Value>> {
    let theme = ["dark", "forest", "neutral", "base", "default"]
        .iter()
        .position(|&theme| theme == name)?;
    let mut variables: BTreeMap<_, _> = COLORS
        .iter()
        .map(|&(key, colors)| (key.to_owned(), Value::from(colors[theme])))
        .collect();
    for (key, start, palette) in [
        ("pie", 1, PIE[theme]),
        ("git", 0, GIT[theme]),
        ("gitBranchLabel", 0, GIT_LABELS[theme]),
        ("gitInv", 0, GIT_HIGHLIGHTS[theme]),
    ] {
        for (index, color) in palette.split_whitespace().enumerate() {
            variables.insert(format!("{key}{}", start + index), Value::from(color));
        }
    }
    if diagram == "radar-beta" {
        for (index, color) in RADAR_CURVES[theme].split_whitespace().enumerate() {
            variables.insert(format!("cScale{index}"), Value::from(color));
        }
        variables.insert("titleColor".into(), Value::from(RADAR_TITLE[theme]));
        variables.insert("radar".into(), json!({ "graticuleColor": "#dedede" }));
    }
    if matches!(diagram, "packet" | "packet-beta") {
        let fg = PACKET_FOREGROUND[theme];
        variables.insert(
            "packet".into(),
            json!({
                "blockStrokeColor": fg,
                "labelColor": fg,
                "startByteColor": fg,
                "endByteColor": fg,
                "titleColor": fg,
            }),
        );
    }
    variables.insert(
        "xyChart".into(),
        json!({
            "plotColorPalette": XY[theme],
            "titleColor": PRIMARY_TEXT[theme],
            "legendTextColor": PRIMARY_TEXT[theme],
        }),
    );
    Some(variables)
}
