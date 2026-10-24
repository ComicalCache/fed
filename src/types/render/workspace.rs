use crate::state::State;
use crate::types::Motion;
use crate::types::Pos;
use crate::types::Rect;
use crate::types::RectSplit;
use crate::types::render::Renderer;
use crate::types::render::Screen;
use crate::types::render::Viewport;
use crate::types::render::ZLayer;
use crate::types::render::workspace::Tile::Dummy;
use crate::util::newtype;

newtype!(WindowId, u64);

enum Tile {
    Split { direction: RectSplit, ratio: f32, first: Box<Self>, second: Box<Self> },
    Window { id: WindowId, content: Box<dyn Renderer> },
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
            Self::Window { id, .. } if *id == target => Some(self),
            Self::Window { .. } => None,
            Self::Dummy => unreachable!(),
        }
    }

    fn find_parent(&mut self, target: WindowId) -> Option<&mut Self> {
        let parent = match &*self {
            Self::Split { first, second, .. } => {
                matches!(**first, Self::Window { id, .. } if id == target)
                    || matches!(**second, Self::Window { id, .. } if id == target)
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

    /// Traverses the tiling tree and collects all window IDs.
    fn windows(&self) -> Vec<WindowId> {
        fn recursive(tile: &Tile, ids: &mut Vec<WindowId>) {
            match tile {
                Tile::Split { first, second, .. } => {
                    recursive(first, ids);
                    recursive(second, ids);
                }
                Tile::Window { id, .. } => ids.push(*id),
                Dummy => unreachable!(),
            }
        }

        let mut ids = Vec::new();
        recursive(self, &mut ids);

        ids
    }

    /// Calculates the layout with physical sizes.
    fn layout(&self, rect: Rect, res: &mut Vec<(WindowId, Rect)>) {
        match self {
            Self::Split { direction, ratio, first, second } => {
                let (first_rect, second_rect) = rect.split(*direction, *ratio);

                first.layout(first_rect, res);
                second.layout(second_rect, res);
            }
            Self::Window { id, .. } => res.push((*id, rect)),
            Self::Dummy => unreachable!(),
        }
    }

    pub fn render(&self, s: &State, screen: &mut Screen, rect: Rect) {
        match self {
            Self::Split { direction, ratio, first, second } => {
                let (first_rect, second_rect) = rect.split(*direction, *ratio);

                first.render(s, screen, first_rect);
                second.render(s, screen, second_rect);
            }
            Self::Window { content, id } => {
                content.render(s, &mut Viewport::new(screen, rect), *id);
            }
            Self::Dummy => unreachable!(),
        }
    }
}

struct Floating {
    rect: Rect,
    z: ZLayer,

    id: WindowId,
    pub content: Box<dyn Renderer>,
}

#[derive(Default)]
pub struct Workspace {
    rect: Rect,

    pub active_window: Option<WindowId>,
    root: Option<Tile>,
    floating: Vec<Floating>,

    next_window_id: u64,
}

impl Workspace {
    pub const fn new(rect: Rect) -> Self {
        Self { rect, active_window: None, root: None, floating: Vec::new(), next_window_id: 0 }
    }

    pub const fn rect(&self) -> Rect { self.rect }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.rect = Rect::new(Pos::default(), width, height);
    }

    /// Collects all tiled window IDs.
    pub fn tiles(&self) -> Vec<WindowId> { self.root.as_ref().map_or_default(Tile::windows) }

    /// Collects all floating window IDs.
    pub fn floatings(&self) -> Vec<WindowId> { self.floating.iter().map(|f| f.id).collect() }

    pub fn create_tile(
        &mut self, split_id: WindowId, direction: RectSplit, content: Box<dyn Renderer>,
    ) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;

        let Some(root) = &mut self.root else {
            self.root = Some(Tile::Window { id, content });

            return id;
        };

        let target = root.find(split_id).unwrap();

        let tile = std::mem::replace(target, Tile::Dummy);

        *target = Tile::Split {
            direction,
            ratio: 0.5,
            first: Box::new(tile),
            second: Box::new(Tile::Window { id, content }),
        };

        id
    }

    pub fn resize_tile(&mut self, id: WindowId, delta: f32) {
        if let Some(root) = &mut self.root
            && let Some(Tile::Split { ratio, .. }) = root.find_parent(id)
        {
            *ratio = (*ratio + delta).clamp(0.05, 0.95);
        }
    }

    pub fn create_floating(
        &mut self, rect: Rect, z: ZLayer, content: Box<dyn Renderer>,
    ) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;

        self.floating.push(Floating { rect, z, id, content });
        self.floating.sort_by_key(|f| f.z);

        id
    }

    pub fn resize_floating(&mut self, id: WindowId, width: usize, height: usize) {
        let floating = self.floating.iter_mut().find(|f| f.id == id).unwrap();
        floating.rect.width = width;
        floating.rect.height = height;
    }

    pub fn reposition_floating(&mut self, id: WindowId, pos: Pos) {
        self.floating.iter_mut().find(|f| f.id == id).unwrap().rect.pos = pos;
    }

    pub fn destroy_window(&mut self, id: WindowId) {
        // Search floating windows first.
        if let Some(idx) = self.floating.iter().position(|f| f.id == id) {
            self.floating.remove(idx);

            if self.active_window == Some(id) {
                self.active_window = None;
            }

            return;
        }

        // Search tiling tree second.
        let root = self.root.as_mut().unwrap();
        match root {
            Tile::Dummy => unreachable!(),
            Tile::Window { id: root, .. } if *root == id => {
                self.root = None;
                if self.active_window == Some(id) {
                    self.active_window = None;
                }
            }
            _ => {
                let parent = root.find_parent(id).unwrap();
                let survivor = if let Tile::Split { first, second, .. } = parent {
                    if matches!(**first, Tile::Window { id: first_id, .. } if first_id == id) {
                        std::mem::replace(&mut **second, Tile::Dummy)
                    } else {
                        std::mem::replace(&mut **first, Tile::Dummy)
                    }
                } else {
                    unreachable!()
                };

                *parent = survivor;

                if self.active_window == Some(id) {
                    self.active_window = None;
                }
            }
        }
    }

    pub fn replace_renderer(&mut self, id: WindowId, content: Box<dyn Renderer>) {
        // Search floating windows first.
        if let Some(idx) = self.floating.iter().position(|f| f.id == id) {
            self.floating[idx].content = content;

            return;
        }

        // Search tiling tree second.
        let root = self.root.as_mut().unwrap();
        let tile = root.find(id).unwrap();
        *tile = Tile::Window { id, content };
    }

    pub fn get_rect(&self, id: WindowId) -> Option<Rect> {
        // Search floating windows first.
        if let Some(f) = self.floating.iter().find(|f| f.id == id) {
            return Some(f.rect);
        }

        // Search tiling tree second.
        if let Some(root) = &self.root {
            let mut layout = Vec::new();
            root.layout(self.rect, &mut layout);

            return layout.into_iter().find(|(w, _)| *w == id).map(|(_, r)| r);
        }

        None
    }

    pub fn get_window(&self, pos: Pos) -> Option<WindowId> {
        // Search floating windows first.
        for floating in self.floating.iter().rev() {
            if floating.rect.contains(pos) {
                return Some(floating.id);
            }
        }

        // Search tiling tree second.
        if let Some(root) = &self.root {
            let mut layout = Vec::new();
            root.layout(self.rect, &mut layout);

            for (id, rect) in layout {
                if rect.contains(pos) {
                    return Some(id);
                }
            }
        }

        None
    }

    pub fn navigate(&self, motion: Motion) -> Option<WindowId> {
        let active = self.active_window?;
        let root = self.root.as_ref()?;

        let mut layout = Vec::new();
        root.layout(self.rect, &mut layout);
        let &(_, active_rect) = layout.iter().find(|(id, _)| *id == active)?;

        let mut res = None;
        let mut min = usize::MAX;
        for (id, rect) in layout {
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
            root.render(s, screen, self.rect);
        }

        for floating in &self.floating {
            floating.content.render(s, &mut Viewport::new(screen, floating.rect), floating.id);
        }
    }
}
