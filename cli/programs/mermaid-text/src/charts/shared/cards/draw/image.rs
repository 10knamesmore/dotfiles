use crate::{color::Rgb, graphics::Scene};

use super::*;
use crate::charts::shared::cards::{End, Mark};

pub(in super::super) const DX: f32 = 16.0;
pub(in super::super) const DY: f32 = 23.0;
const PAD: f32 = 22.0;
const PANEL_GAP: f32 = 27.0;

type Point = (f32, f32);

pub(in super::super) fn image(
    diagram: &Diagram<'_>,
    options: &Options,
    scene: &mut Scene,
) -> Result<(), String> {
    let layout = Layout::new(diagram, options);
    let layout = Layout {
        width: layout.width.clamp(25, 92),
        ..layout
    };
    let mut drawing = Image {
        diagram,
        layout,
        scene,
        x: PAD,
        y: PAD,
        row_height: 0.0,
        size: (2.0 * PAD, 2.0 * PAD),
    };
    drawing.headings()?;
    for component in components(diagram) {
        if component.edges.is_empty() {
            drawing.card(component.nodes[0])?;
            continue;
        }
        let mut remaining = component.edges.as_slice();
        let panel = if component.nodes.len() <= 64 && component.edges.len() <= 96 {
            let levels = levels(diagram, &component, layout.reverse);
            layout.panel(diagram, &component.edges, &levels, false)
        } else {
            None
        };
        if let Some(panel) = &panel {
            drawing.panel(panel)?;
            remaining = &panel.remaining;
        } else {
            for &index in &component.nodes {
                drawing.card(index)?;
            }
        }
        if !remaining.is_empty() {
            drawing.heading("Connections")?;
            for &index in remaining {
                drawing.connection(&diagram.relationships[index])?;
            }
        }
    }
    drawing.scene.resize(drawing.size.0, drawing.size.1)
}

struct Image<'a, 'source> {
    diagram: &'a Diagram<'source>,
    layout: Layout<'a>,
    scene: &'a mut Scene,
    x: f32,
    y: f32,
    row_height: f32,
    size: Point,
}

impl Image<'_, '_> {
    fn newline(&mut self) {
        self.x = PAD;
        if self.row_height > 0.0 {
            self.y += self.row_height + PANEL_GAP;
            self.row_height = 0.0;
        }
    }

    fn reserve(&mut self, (w, h): Point, owns_row: bool) -> Result<Point, String> {
        if owns_row || (self.x > PAD && self.x + w > PAD + self.layout.width as f32 * DX) {
            self.newline();
        }
        let origin = (self.x, self.y);
        self.size = (
            self.size.0.max(self.x + w + PAD),
            self.size.1.max(self.y + h + PAD),
        );
        self.scene.resize(self.size.0, self.size.1)?;
        self.x += w + PANEL_GAP;
        self.row_height = self.row_height.max(h);
        if owns_row {
            self.newline();
        }
        Ok(origin)
    }

    fn heading(&mut self, text: &str) -> Result<(), String> {
        let lines = wrap(text, self.layout.width.min(60));
        let (x, y) = self.reserve((measure(self.scene, &lines), lines.len() as f32 * DY), true)?;
        for (i, line) in lines.iter().enumerate() {
            self.scene.text(x, y + i as f32 * DY, line, None);
        }
        Ok(())
    }

    fn headings(&mut self) -> Result<(), String> {
        if let Some(title) = self.diagram.title {
            self.heading(title)?;
        }
        for note in &self.diagram.notes {
            self.heading(&format!(
                "Note: {}",
                display_text(note).replace("\\n", "\n")
            ))?;
        }
        let mut occupied = vec![false; self.diagram.namespaces.len()];
        for node in &self.diagram.nodes {
            if let Some(i) = node.namespace {
                occupied[i] = true;
            }
        }
        for (i, namespace) in self.diagram.namespaces.iter().enumerate().rev() {
            if !occupied[i] {
                let mut scope = Some(i);
                let mut path = Vec::new();
                while let Some(index) = scope {
                    let namespace = &self.diagram.namespaces[index];
                    path.push(namespace.label.unwrap_or(namespace.name));
                    scope = namespace.parent;
                }
                path.reverse();
                self.heading(&format!("Namespace: {}", path.join(" / ")))?;
            }
            if let Some(parent) = namespace.parent {
                occupied[parent] = true;
            }
        }
        Ok(())
    }

    fn card(&mut self, index: usize) -> Result<(), String> {
        let card = Card::new(
            self.diagram,
            index,
            self.layout.width.min(44),
            self.layout.options,
        );
        let origin = self.reserve(
            (
                (card.width - 1) as f32 * DX,
                (card.height() - 1) as f32 * DY,
            ),
            false,
        )?;
        card.paint_image(self.scene, origin, self.diagram.nodes[index].style);
        Ok(())
    }

    fn panel(&mut self, panel: &Panel) -> Result<(), String> {
        let (x, y) = self.reserve(
            (
                (panel.size.0 - 1) as f32 * DX,
                (panel.size.1 - 1) as f32 * DY,
            ),
            true,
        )?;
        let pixel = |(a, b): (usize, usize)| (x + a as f32 * DX, y + b as f32 * DY);
        let line = self
            .layout
            .options
            .theme_line()
            .unwrap_or_else(|| self.scene.ink());
        let label_color = self.layout.options.theme_style().text;
        for (index, rect, card) in &panel.cards {
            card.paint_image(
                self.scene,
                pixel((rect.x, rect.y)),
                self.diagram.nodes[*index].style,
            );
        }
        for (i, wire, reversed) in &panel.wires {
            let edge = &self.diagram.relationships[*i];
            let mut ends = edge.ends;
            if *reversed {
                ends.reverse();
            }
            paint_link(self.scene, wire.points().map(pixel), edge, ends, line);
            for (u, v, text) in wire.captions() {
                let (x, y) = pixel((u, v));
                self.scene.text(x, y - DY / 2.0, text, label_color);
            }
        }
        Ok(())
    }

    fn connection(&mut self, edge: &Relationship<'_>) -> Result<(), String> {
        if edge.left == edge.right {
            return self.loop_link(edge);
        }
        let mut indices = [edge.left, edge.right];
        let mut ends = edge.ends;
        if self.layout.reverse {
            indices.reverse();
            ends.reverse();
        }
        let cards = indices.map(|i| Card::reference(self.diagram, i, 32, self.layout.options));
        let [a, b] = cards.each_ref().map(|card| (card.width - 1) as f32 * DX);
        let card_height = cards.iter().map(Card::height).max().unwrap_or(2);
        let label = wrap(
            &self.layout.label(self.diagram, edge, self.layout.reverse),
            44,
        );
        let counts = ends.map(|end| end.count.map(|count| wrap(count, 24)).unwrap_or_default());
        let count_height = counts.iter().map(Vec::len).max().unwrap_or(0) as f32 * DY;
        let gap = (measure(self.scene, &counts[0]) + measure(self.scene, &counts[1]) + 56.0)
            .max(measure(self.scene, &label) + 32.0)
            .max(180.0);
        let top = label.len() as f32 * DY;
        let height = ((card_height - 1) as f32 * DY).max(DY + count_height);
        let (x, y) = self.reserve((a + gap + b, top + height), true)?;
        let left = x + a;
        let right = left + gap;
        let port_y = y + top + DY;
        let line = self
            .layout
            .options
            .theme_line()
            .unwrap_or_else(|| self.scene.ink());
        let label_color = self.layout.options.theme_style().text;
        for (((px, card), index), count) in [(x, &cards[0]), (right, &cards[1])]
            .into_iter()
            .zip(indices)
            .zip(&counts)
        {
            card.paint_image(self.scene, (px, y + top), self.diagram.nodes[index].style);
            for (i, text) in count.iter().enumerate() {
                let start = if px == x {
                    left + 22.0
                } else {
                    right - 22.0 - self.scene.measure(text)
                };
                self.scene
                    .text(start, port_y + 4.0 + i as f32 * DY, text, label_color);
            }
        }
        for (i, text) in label.iter().enumerate() {
            let text_width = self.scene.measure(text);
            self.scene.text(
                left + (gap - text_width) / 2.0,
                y + i as f32 * DY,
                text,
                label_color,
            );
        }
        paint_link(
            self.scene,
            [
                (left, port_y),
                ((left + right) / 2.0, port_y),
                ((left + right) / 2.0, port_y),
                (right, port_y),
            ],
            edge,
            ends,
            line,
        );
        Ok(())
    }

    fn loop_link(&mut self, edge: &Relationship<'_>) -> Result<(), String> {
        let card = Card::reference(self.diagram, edge.left, 32, self.layout.options);
        let card_size = (
            (card.width - 1) as f32 * DX,
            (card.height() - 1) as f32 * DY,
        );
        let label = wrap(&self.layout.label(self.diagram, edge, false), 44);
        let counts = edge
            .ends
            .map(|end| end.count.map(|count| wrap(count, 24)).unwrap_or_default());
        let [upper, lower] = counts.each_ref().map(|rows| rows.len() as f32 * DY);
        let label_height = label.len() as f32 * DY;
        let gap = counts
            .iter()
            .map(|rows| measure(self.scene, rows))
            .fold(64.0, f32::max)
            + 24.0;
        let extra_top = (upper - 10.0).max(0.0);
        let height = extra_top + card_size.1 + (lower - 10.0).max(0.0);
        let (x, y) = self.reserve(
            (
                card_size.0 + gap + measure(self.scene, &label) + 12.0,
                height.max(label_height),
            ),
            true,
        )?;
        let top = y + extra_top;
        card.paint_image(self.scene, (x, top), self.diagram.nodes[edge.left].style);
        let a = x + card_size.0;
        let b = a + gap;
        let (first, last) = (top + 14.0, top + card_size.1 - 14.0);
        let line = self
            .layout
            .options
            .theme_line()
            .unwrap_or_else(|| self.scene.ink());
        let ink = self.layout.options.theme_style().text;
        paint_link(
            self.scene,
            [(a, first), (b, first), (b, last), (a, last)],
            edge,
            edge.ends,
            line,
        );
        for (rows, start) in [(&counts[0], first - 4.0 - upper), (&counts[1], last + 4.0)] {
            for (i, text) in rows.iter().enumerate() {
                self.scene.text(a + 13.0, start + i as f32 * DY, text, ink);
            }
        }
        let label_y = ((first + last - label_height) / 2.0).max(y);
        for (i, text) in label.iter().enumerate() {
            self.scene
                .text(b + 12.0, label_y + i as f32 * DY, text, ink);
        }
        Ok(())
    }
}

fn measure(scene: &Scene, lines: &[String]) -> f32 {
    lines.iter().map(|s| scene.measure(s)).fold(0.0, f32::max)
}

pub(in super::super) fn stroke(
    scene: &mut Scene,
    points: &[Point],
    ink: Rgb,
    width: f32,
    dashed: bool,
) {
    let mut path = String::new();
    if dashed {
        for pair in points.windows(2) {
            let [(x, y), (u, v)] = [pair[0], pair[1]];
            let len = (u - x).hypot(v - y);
            let mut offset = 0.0_f32;
            while offset < len {
                let a = offset / len;
                let b = (offset + 6.0).min(len) / len;
                let _ = write!(
                    path,
                    "M{} {}L{} {}",
                    x + (u - x) * a,
                    y + (v - y) * a,
                    x + (u - x) * b,
                    y + (v - y) * b
                );
                offset += 10.0;
            }
        }
    } else {
        for (i, (x, y)) in points.iter().enumerate() {
            let command = if i == 0 { 'M' } else { 'L' };
            let _ = write!(path, "{command}{x} {y}");
        }
    }
    scene.path(&path, None, Some(ink), width);
}

fn paint_link(
    scene: &mut Scene,
    points: [Point; 4],
    edge: &Relationship<'_>,
    ends: [End<'_>; 2],
    ink: Rgb,
) {
    stroke(scene, &points, ink, 1.55, matches!(edge.link, Link::Dashed));
    for (end, (at, toward)) in ends
        .into_iter()
        .zip([(points[0], points[1]), (points[3], points[2])])
    {
        let length = (toward.0 - at.0).hypot(toward.1 - at.1);
        if length > 0.0 {
            let direction = ((toward.0 - at.0) / length, (toward.1 - at.1) / length);
            let shape = EndShape { at, direction, ink };
            if edge.directional_label {
                shape.cardinality(scene, end.count);
            } else {
                shape.marker(scene, end.mark);
            }
        }
    }
}

struct EndShape {
    at: Point,
    direction: Point,
    ink: Rgb,
}

impl EndShape {
    fn point(&self, (along, across): Point) -> Point {
        let ((x, y), (u, v)) = (self.at, self.direction);
        (x + u * along - v * across, y + v * along + u * across)
    }

    fn polygon(&self, scene: &mut Scene, points: &[Point], filled: bool) {
        let mut path = String::new();
        for (i, &point) in points.iter().enumerate() {
            let (x, y) = self.point(point);
            let command = if i == 0 { 'M' } else { 'L' };
            let _ = write!(path, "{command}{x} {y}");
        }
        path.push('Z');
        scene.path(
            &path,
            Some(if filled { self.ink } else { scene.paper() }),
            Some(self.ink),
            1.55,
        );
    }

    fn circle(&self, scene: &mut Scene, center: f32) {
        let (x, y) = self.point((center, 0.0));
        let (a, b) = (x - 3.8, x + 3.8);
        scene.path(
            &format!("M{a} {y}A3.8 3.8 0 1 0 {b} {y}A3.8 3.8 0 1 0 {a} {y}Z"),
            Some(scene.paper()),
            Some(self.ink),
            1.55,
        );
    }

    fn marker(&self, scene: &mut Scene, mark: Mark) {
        match mark {
            Mark::None => {}
            Mark::Circle => self.circle(scene, 4.5),
            Mark::Triangle => self.polygon(scene, &[(0.0, 0.0), (11.0, -5.6), (11.0, 5.6)], false),
            Mark::Diamond | Mark::HollowDiamond => self.polygon(
                scene,
                &[(0.0, 0.0), (7.0, -5.0), (14.0, 0.0), (7.0, 5.0)],
                matches!(mark, Mark::Diamond),
            ),
            Mark::Arrow => self.line(scene, [(8.0, -4.5), (0.0, 0.0), (8.0, 4.5)]),
        }
    }

    fn line<const N: usize>(&self, scene: &mut Scene, points: [Point; N]) {
        stroke(
            scene,
            &points.map(|point| self.point(point)),
            self.ink,
            1.55,
            false,
        );
    }

    fn cardinality(&self, scene: &mut Scene, count: Option<&str>) {
        let Some(count @ ("1" | "0..1" | "1..*" | "0..*")) = count else {
            return;
        };
        if count.ends_with('*') {
            self.line(scene, [(0.0, -5.0), (10.0, 0.0), (0.0, 5.0)]);
        } else {
            self.line(scene, [(4.0, -5.0), (4.0, 5.0)]);
        }
        if count.starts_with('0') {
            self.circle(scene, 16.0);
        } else {
            self.line(scene, [(13.0, -5.0), (13.0, 5.0)]);
        }
    }
}
