use std::cell::OnceCell;

use crate::state::State;
use crate::types::Motion;
use crate::types::Pos;
use crate::types::Rect;
use crate::types::RectSplit;
use crate::types::render::Renderer;
use crate::types::render::Screen;
use crate::types::render::Viewport;
use crate::types::render::ZLayer;
use crate::util::newtype;

newtype!(WindowId, u64);

enum Tile {
    Split { direction: RectSplit, ratio: f32, first: Box<Self>, second: Box<Self> },
    Window { window: WindowId, content: Box<dyn Renderer> },
    Dummy,
}

impl Tile {
    fn find(&mut self, target: WindowId) -> Option<&mut Self> {
        match self {
            Self::Split { first, second, .. } => {
                if let Some(tile) = first.find(target) {
                    return Some(tile);
                }

                second.find(target)
            }
            Self::Window { window, .. } if *window == target => Some(self),
            Self::Window { .. } => None,
            Self::Dummy => unreachable!(),
        }
    }

    fn find_parent(&mut self, target: WindowId) -> Option<&mut Self> {
        let parent = match &*self {
            Self::Split { first, second, .. } => {
                matches!(**first, Self::Window { window, .. } if window == target)
                    || matches!(**second, Self::Window { window, .. } if window == target)
            }
            Self::Window { .. } => false,
            Self::Dummy => unreachable!(),
        };

        if parent {
            return Some(self);
        }

        match self {
            Self::Split { first, second, .. } => {
                if let Some(parent) = first.find_parent(target) {
                    return Some(parent);
                }

                second.find_parent(target)
            }
            Self::Window { .. } => None,
            Self::Dummy => unreachable!(),
        }
    }

    /// Calculates the layout with physical sizes.
    fn layout(&self, rect: Rect, res: &mut Vec<(WindowId, Rect)>) {
        match self {
            Self::Split { direction, ratio, first, second } => {
                let (first_rect, second_rect) = rect.split(*direction, *ratio);

                first.layout(first_rect, res);
                second.layout(second_rect, res);
            }
            Self::Window { window, .. } => res.push((*window, rect)),
            Self::Dummy => unreachable!(),
        }
    }

    fn render(
        &self, s: &State, screen: &mut Screen, layout: &mut std::slice::Iter<'_, (WindowId, Rect)>,
    ) {
        match self {
            Self::Split { first, second, .. } => {
                first.render(s, screen, layout);
                second.render(s, screen, layout);
            }
            Self::Window { content, window } => {
                let &(id, rect) = layout.next().unwrap();
                debug_assert!(id == *window);

                content.render(s, &mut Viewport::new(screen, rect), *window);
            }
            Self::Dummy => unreachable!(),
        }
    }
}

struct Floating {
    rect: Rect,
    z: ZLayer,

    window: WindowId,
    pub content: Box<dyn Renderer>,
}

#[derive(Default)]
pub struct Workspace {
    rect: Rect,

    pub active_window: Option<WindowId>,
    root: Option<Tile>,
    floating: Vec<Floating>,

    layout: OnceCell<Vec<(WindowId, Rect)>>,

    next_window_id: u64,
}

impl Workspace {
    pub const fn new(rect: Rect) -> Self {
        Self {
            rect,
            active_window: None,
            root: None,
            floating: Vec::new(),
            layout: OnceCell::new(),
            next_window_id: 0,
        }
    }

    fn layout(&self) -> &[(WindowId, Rect)] {
        self.layout.get_or_init(|| {
            let mut layout = Vec::new();

            if let Some(root) = &self.root {
                root.layout(self.rect, &mut layout);
            }

            layout
        })
    }

    pub const fn rect(&self) -> Rect { self.rect }

    pub fn resize(&mut self, width: usize, height: usize) {
        if self.rect.pos != Pos::default() || self.rect.width != width || self.rect.height != height
        {
            self.rect = Rect::new(Pos::default(), width, height);
            self.layout.take();
        }
    }

    /// Collects all tiled window IDs.
    pub fn tiles(&self) -> Vec<WindowId> { self.layout().iter().map(|(id, _)| *id).collect() }

    /// Collects all floating window IDs.
    pub fn floatings(&self) -> Vec<WindowId> { self.floating.iter().map(|f| f.window).collect() }

    pub fn create_tile(
        &mut self, split_id: WindowId, direction: RectSplit, content: Box<dyn Renderer>,
    ) -> WindowId {
        let window = WindowId(self.next_window_id);
        self.next_window_id += 1;

        let Some(root) = &mut self.root else {
            self.root = Some(Tile::Window { window, content });
            self.layout.take();

            return window;
        };

        let target = root.find(split_id).unwrap();

        let tile = std::mem::replace(target, Tile::Dummy);

        *target = Tile::Split {
            direction,
            ratio: 0.5,
            first: Box::new(tile),
            second: Box::new(Tile::Window { window, content }),
        };
        self.layout.take();

        window
    }

    pub fn resize_tile(&mut self, window: WindowId, delta: f32) {
        if let Some(root) = &mut self.root
            && let Some(Tile::Split { ratio, .. }) = root.find_parent(window)
        {
            let resized = (*ratio + delta).clamp(0.05, 0.95);
            if *ratio != resized {
                *ratio = resized;
                self.layout.take();
            }
        }
    }

    pub fn create_floating(
        &mut self, rect: Rect, z: ZLayer, content: Box<dyn Renderer>,
    ) -> WindowId {
        let window = WindowId(self.next_window_id);
        self.next_window_id += 1;

        self.floating.push(Floating { rect, z, window, content });
        self.floating.sort_by_key(|f| f.z);

        window
    }

    pub fn resize_floating(&mut self, window: WindowId, width: usize, height: usize) {
        let floating = self.floating.iter_mut().find(|f| f.window == window).unwrap();
        floating.rect.width = width;
        floating.rect.height = height;
    }

    pub fn reposition_floating(&mut self, window: WindowId, pos: Pos) {
        self.floating.iter_mut().find(|f| f.window == window).unwrap().rect.pos = pos;
    }

    pub fn destroy_window(&mut self, window: WindowId) {
        if let Some(idx) = self.floating.iter().position(|f| f.window == window) {
            self.floating.remove(idx);

            if self.active_window == Some(window) {
                self.active_window = None;
            }

            return;
        }

        let root = self.root.as_mut().unwrap();
        match root {
            Tile::Dummy => unreachable!(),
            Tile::Window { window: root, .. } if *root == window => {
                self.root = None;
                if self.active_window == Some(window) {
                    self.active_window = None;
                }
            }
            _ => {
                let parent = root.find_parent(window).unwrap();
                let survivor = if let Tile::Split { first, second, .. } = parent {
                    if matches!(**first, Tile::Window { window: first_id, .. } if first_id == window)
                    {
                        std::mem::replace(&mut **second, Tile::Dummy)
                    } else {
                        std::mem::replace(&mut **first, Tile::Dummy)
                    }
                } else {
                    unreachable!()
                };

                *parent = survivor;

                if self.active_window == Some(window) {
                    self.active_window = None;
                }
            }
        }
        self.layout.take();
    }

    pub fn replace_renderer(&mut self, window: WindowId, content: Box<dyn Renderer>) {
        if let Some(idx) = self.floating.iter().position(|f| f.window == window) {
            self.floating[idx].content = content;

            return;
        }

        let root = self.root.as_mut().unwrap();
        let tile = root.find(window).unwrap();
        *tile = Tile::Window { window, content };
    }

    pub fn get_rect(&self, window: WindowId) -> Option<Rect> {
        if let Some(f) = self.floating.iter().find(|f| f.window == window) {
            return Some(f.rect);
        }

        self.layout().iter().find(|(w, _)| *w == window).map(|(_, r)| *r)
    }

    pub fn get_window(&self, pos: Pos) -> Option<WindowId> {
        for floating in self.floating.iter().rev() {
            if floating.rect.contains(pos) {
                return Some(floating.window);
            }
        }

        self.layout().iter().find(|(_, rect)| rect.contains(pos)).map(|(id, _)| *id)
    }

    pub fn navigate(&self, motion: Motion) -> Option<WindowId> {
        let active = self.active_window?;
        let layout = self.layout();
        let &(_, active_rect) = layout.iter().find(|(id, _)| *id == active)?;

        let mut res = None;
        let mut min = usize::MAX;
        for &(id, rect) in layout {
            if id == active {
                continue;
            }

            let (x, y) = rect.intersects(active_rect);
            let valid = match motion {
                Motion::Left => rect.pos.x < active_rect.pos.x && y,
                Motion::Right => rect.pos.x > active_rect.pos.x && y,
                Motion::Up => rect.pos.y < active_rect.pos.y && x,
                Motion::Down => rect.pos.y > active_rect.pos.y && x,
                _ => unreachable!(),
            };

            if valid {
                let dist = rect.manhattan_distance(active_rect);
                if dist < min {
                    min = dist;
                    res = Some(id);
                }
            }
        }

        res
    }

    pub fn render(&self, s: &State, screen: &mut Screen) {
        if let Some(root) = &self.root {
            root.render(s, screen, &mut self.layout().iter());
        }

        for floating in &self.floating {
            floating.content.render(s, &mut Viewport::new(screen, floating.rect), floating.window);
        }
    }
}
