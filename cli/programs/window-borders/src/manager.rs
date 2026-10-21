use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};

use crate::{
    color::Color,
    geometry::{BorderGeometry, Layout},
    macos::{
        Connection, Placement, Surface, TargetWindow, WindowId, displays::Displays, events::Changes,
    },
    paint::{BorderPaint, TopSegments},
    stack::FocusedStack,
};

pub struct Settings {
    pub selected_window: Option<WindowId>,
    pub layout: Layout,
    pub width: f64,
    pub radius: f64,
    pub scale_override: Option<f64>,
    pub active_color: Color,
    pub inactive_color: Color,
    pub verbose: bool,
}

struct Border {
    geometry: BorderGeometry,
    paint: BorderPaint,
    surfaces: Vec<Surface>,
    placement: Placement,
}

struct ManagedWindow {
    state: TargetWindow,
    border: Option<Border>,
}

pub struct WindowManager {
    connection: Connection,
    settings: Settings,
    displays: Displays,
    windows: BTreeMap<WindowId, ManagedWindow>,
    focused: Option<WindowId>,
    stack: Option<FocusedStack>,
    pub updates: u64,
}

impl WindowManager {
    pub fn new(connection: Connection, settings: Settings) -> Result<Self> {
        let displays = Displays::read(connection)?;
        let mut manager = Self {
            connection,
            settings,
            displays,
            windows: BTreeMap::new(),
            focused: None,
            stack: None,
            updates: 0,
        };
        manager.refresh(Changes {
            spaces: true,
            focus: true,
            ..Changes::default()
        })?;
        if let Some(id) = manager.settings.selected_window {
            ensure!(
                manager.windows.contains_key(&id),
                "window {id} is not an available document or modal window"
            );
        }
        println!(
            "pid={} managed={} visible={} surfaces={}",
            std::process::id(),
            manager.windows.len(),
            manager
                .windows
                .values()
                .filter(|window| window.border.is_some())
                .count(),
            manager
                .windows
                .values()
                .filter_map(|window| window.border.as_ref())
                .map(|border| border.surfaces.len())
                .sum::<usize>()
        );
        Ok(manager)
    }

    pub fn refresh(&mut self, changes: Changes) -> Result<bool> {
        if self
            .settings
            .selected_window
            .is_some_and(|id| changes.closed.contains(&id))
        {
            return Ok(false);
        }
        let previous_ids = self.windows.keys().copied().collect::<BTreeSet<_>>();
        let mut affected = changes
            .changed
            .intersection(&previous_ids)
            .copied()
            .collect::<BTreeSet<_>>();
        affected.extend(changes.created);
        let refresh_displays = changes.spaces || changes.displays;
        if refresh_displays {
            self.displays = Displays::read(self.connection)?;
            affected.extend(self.connection.discover_windows()?);
            affected.extend(&previous_ids);
            if self.settings.verbose {
                for screen in &self.displays.screens {
                    println!(
                        "display={} scale={} space={} fullscreen={}",
                        screen.id, screen.scale, screen.space, screen.fullscreen
                    );
                }
            }
        }
        if changes.focus || refresh_displays {
            let focused = self.connection.focused_window(&self.displays)?;
            if focused != self.focused {
                affected.extend(self.focused);
                affected.extend(focused);
                if let Some(stack) = &self.stack {
                    affected.extend(&stack.members);
                    if let (Some(previous), Some(next)) = (self.focused, focused)
                        && stack.members.contains(&previous)
                        && stack.members.contains(&next)
                    {
                        // A stack shares one border group. Retarget its surfaces
                        // when focus changes instead of reallocating four windows.
                        let border = self
                            .windows
                            .get_mut(&previous)
                            .and_then(|window| window.border.take());
                        if let Some(window) = self.windows.get_mut(&next) {
                            window.border = border;
                        }
                    }
                }
                if self.settings.verbose {
                    println!("focus={focused:?}");
                }
                self.focused = focused;
            }
        }
        for id in changes.closed {
            affected.remove(&id);
            if self.windows.remove(&id).is_some() && self.settings.verbose {
                println!("remove target={id}");
            }
        }
        if let Some(selected) = self.settings.selected_window {
            affected.retain(|&id| id == selected);
        }
        for id in affected {
            let state = self
                .connection
                .target(id)
                .with_context(|| format!("refresh target {id}"))?;
            if let Some(state) = state {
                // Fullscreen transitions can temporarily change a tracked
                // window's role/attributes. Keep its identity until it closes.
                if !state.eligible && !self.windows.contains_key(&id) {
                    continue;
                }
                if let Some(window) = self.windows.get_mut(&id) {
                    window.state = state;
                } else {
                    if self.settings.verbose {
                        println!("track target={id}");
                    }
                    self.windows.insert(
                        id,
                        ManagedWindow {
                            state,
                            border: None,
                        },
                    );
                }
                self.render(id)
                    .with_context(|| format!("render target {id}"))?;
            } else {
                if self.windows.remove(&id).is_some() && self.settings.verbose {
                    println!("remove target={id}");
                }
                if self.settings.selected_window == Some(id) {
                    return Ok(false);
                }
            }
        }
        let ids = self.windows.keys().copied().collect::<BTreeSet<_>>();
        if ids != previous_ids {
            self.connection
                .watch_windows(&ids.into_iter().collect::<Vec<_>>())?;
        }
        self.updates += 1;
        Ok(true)
    }

    pub fn update_stack(&mut self, stack: Option<FocusedStack>) -> Result<()> {
        if self.stack == stack {
            return Ok(());
        }
        let mut affected = BTreeSet::new();
        for stack in self.stack.iter().chain(stack.iter()) {
            affected.extend(stack.members.iter().copied());
        }
        if self.settings.verbose {
            println!("stack={stack:?}");
        }
        self.stack = stack;
        for id in affected {
            if self.windows.contains_key(&id) {
                self.render(id)?;
            }
        }
        Ok(())
    }

    fn render(&mut self, id: WindowId) -> Result<()> {
        let window = self.windows.get_mut(&id).unwrap();
        let stack = self
            .stack
            .as_ref()
            .filter(|stack| self.focused.is_some_and(|id| stack.members.contains(&id)));
        // Lower stack members would otherwise show through the top-edge gaps.
        if stack.is_some_and(|stack| self.focused != Some(id) && stack.members.contains(&id)) {
            if window.border.take().is_some() && self.settings.verbose {
                println!("release target={id} reason=covered-by-stack");
            }
            return Ok(());
        }
        let display = self.displays.for_window(&window.state);
        let display = display
            .filter(|_| window.state.eligible && window.state.shown && !window.state.fullscreen);
        let Some(display) = display else {
            if window.border.take().is_some() && self.settings.verbose {
                println!("release target={id} reason=not-visible-on-desktop");
            }
            return Ok(());
        };
        let color = if self.focused == Some(id) {
            self.settings.active_color
        } else {
            self.settings.inactive_color
        };
        let paint = BorderPaint {
            color,
            top_segments: stack
                .filter(|_| self.focused == Some(id))
                .map(|stack| TopSegments {
                    count: stack.members.len(),
                    active_index: stack
                        .members
                        .iter()
                        .position(|&member| member == id)
                        .unwrap(),
                    inactive_color: self.settings.inactive_color,
                }),
        };
        let scale = self.settings.scale_override.unwrap_or(display.scale);
        let geometry = BorderGeometry::new(
            window.state.frame,
            self.settings.layout,
            self.settings.width,
            self.settings.radius,
            scale,
        );
        let placement = Placement {
            target: id,
            space: display.space,
            level: window.state.level,
            sublevel: window.state.sublevel,
            sticky: window.state.sticky,
        };
        let replace = window.border.as_ref().is_none_or(|border| {
            border.geometry.surfaces != geometry.surfaces
                || border.geometry.inner != geometry.inner
                || border.geometry.scale != geometry.scale
        });
        if replace {
            let surfaces = geometry
                .surfaces
                .iter()
                .map(|&viewport| Surface::draw(self.connection, &geometry, viewport, paint))
                .collect::<Result<Vec<_>>>()?;
            let retiring = window
                .border
                .as_ref()
                .map_or(&[][..], |border| &border.surfaces);
            self.connection.set_membership(&surfaces, &placement)?;
            self.connection
                .place(&surfaces, retiring, &geometry, &placement)?;
            if self.settings.verbose {
                println!(
                    "replace target={id} size={}x{} scale={scale} color={color} rgba8_bytes={} ids={:?}",
                    window.state.frame.size.width,
                    window.state.frame.size.height,
                    geometry.rgba8_bytes(),
                    surfaces
                        .iter()
                        .map(|surface| surface.id)
                        .collect::<Vec<_>>()
                );
            }
            window.border = Some(Border {
                geometry,
                paint,
                surfaces,
                placement,
            });
        } else {
            let border = window.border.as_mut().unwrap();
            if border.placement.space != placement.space
                || border.placement.sticky != placement.sticky
            {
                self.connection
                    .set_membership(&border.surfaces, &placement)?;
            }
            if border.paint != paint {
                for (surface, &viewport) in border.surfaces.iter().zip(&geometry.surfaces) {
                    if border.paint.color != paint.color
                        || viewport.origin.y < geometry.inner.origin.y
                    {
                        surface.repaint(&geometry, viewport, paint)?;
                    }
                }
                border.paint = paint;
                if self.settings.verbose {
                    println!("repaint target={id} paint={paint:?}");
                }
            }
            self.connection
                .place(&border.surfaces, &[], &geometry, &placement)?;
            border.geometry = geometry;
            border.placement = placement;
        }
        Ok(())
    }
}
