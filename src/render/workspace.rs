use crate::{
    newtype::newtype,
    render::{Renderer, Screen, Viewport, ZLayer, workspace::Tile::Dummy},
    state::State,
    types::{Direction, Pos, Rect, RectSplit},
};

newtype!(WindowId, u64);

/// A window tile in the tiling window of the workspace of the editor.
enum Tile {
    Split { direction: RectSplit, ratio: f32, first: Box<Tile>, second: Box<Tile> },
    Window { id: WindowId, content: Box<dyn Renderer> },
    Dummy,
}

impl Tile {
    fn find(&mut self, target: WindowId) -> Option<&mut Tile> {
        match self {
            Tile::Split { first, second, .. } => {
                if let Some(tile) = first.find(target) {
                    return Some(tile);
                }

                second.find(target)
            }
            Tile::Window { id, .. } if *id == target => Some(self),
            _ => None,
        }
    }

    fn find_parent(&mut self, target: WindowId) -> Option<&mut Tile> {
        let self_parent = match &*self {
            Tile::Split { first, second, .. } => {
                matches!(**first, Tile::Window { id, .. } if id == target)
                    || matches!(**second, Tile::Window { id, .. } if id == target)
            }
            _ => false,
        };

        if self_parent {
            return Some(self);
        }

        match self {
            Tile::Split { first, second, .. } => {
                if let Some(parent) = first.find_parent(target) {
                    return Some(parent);
                }

                second.find_parent(target)
            }
            _ => None,
        }
    }

    /// Traverses the tiling tree and collects all window IDs.
    fn windows(&self) -> Vec<WindowId> {
        let mut ids = Vec::new();

        fn recursive(tile: &Tile, ids: &mut Vec<WindowId>) {
            match tile {
                Tile::Split { first, second, .. } => {
                    recursive(first, ids);
                    recursive(second, ids);
                }
                Tile::Window { id, .. } => ids.push(*id),
                Dummy => {}
            }
        }

        recursive(self, &mut ids);

        ids
    }

    /// Calculates the layout with physical sizes.
    fn layout(&self, rect: Rect, res: &mut Vec<(WindowId, Rect)>) {
        match self {
            Tile::Split { direction, ratio, first, second } => {
                let (first_rect, second_rect) = rect.split(*direction, *ratio);

                first.layout(first_rect, res);
                second.layout(second_rect, res);
            }
            Tile::Window { id, .. } => res.push((*id, rect)),
            Tile::Dummy => {}
        }
    }

    pub fn render(&self, state: &State, screen: &mut Screen, rect: Rect) {
        match self {
            Tile::Split { direction, ratio, first, second } => {
                let (first_rect, second_rect) = rect.split(*direction, *ratio);

                first.render(state, screen, first_rect);
                second.render(state, screen, second_rect);
            }
            Tile::Window { content, id } => {
                content.render(state, &mut Viewport::new(screen, rect), *id);
            }
            Tile::Dummy => unreachable!(),
        }
    }
}

/// A floating window in the workspace of the editor. It "floats" above the
/// tiling windows.
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
    pub fn new(rect: Rect) -> Self {
        Self { rect, active_window: None, root: None, floating: Vec::new(), next_window_id: 0 }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.rect = Rect::new(Pos::default(), width, height);
    }

    /// Collects all tiled window IDs.
    pub fn tiles(&self) -> Vec<WindowId> {
        let mut ids = Vec::new();

        if let Some(tile) = &self.root {
            ids.extend(tile.windows());
        }

        ids
    }

    /// Collects all floating window IDs.
    pub fn floatings(&self) -> Vec<WindowId> {
        self.floating.iter().map(|floating| floating.id).collect()
    }

    pub fn create_tile(&mut self, direction: RectSplit, content: Box<dyn Renderer>) -> WindowId {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;

        if let Some(root) = &mut self.root
            && let Some(active_window) = self.active_window
            && let Some(target) = root.find(active_window)
        {
            let tile = std::mem::replace(target, Tile::Dummy);

            *target = Tile::Split {
                direction,
                ratio: 0.5,
                first: Box::new(tile),
                second: Box::new(Tile::Window { id, content }),
            };

            return id;
        }

        self.root = Some(Tile::Window { id, content });

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
        self.floating.sort_by_key(|floating| floating.z);

        id
    }

    pub fn resize_floating(&mut self, id: WindowId, width: usize, height: usize) {
        if let Some(floating) = self.floating.iter_mut().find(|floating| floating.id == id) {
            floating.rect.width = width;
            floating.rect.height = height;
        }
    }

    pub fn reposition_floating(&mut self, id: WindowId, pos: Pos) {
        if let Some(idx) = self.floating.iter().position(|floating| floating.id == id) {
            self.floating[idx].rect.pos = pos;
        }
    }

    pub fn destroy_window(&mut self, id: WindowId) {
        // Search floating windows first.
        if let Some(idx) = self.floating.iter().position(|floating| floating.id == id) {
            self.floating.remove(idx);

            if self.active_window == Some(id) {
                self.active_window = None;
            }

            return;
        }

        // Search tiling tree second.
        let Some(root) = &mut self.root else { return };
        match root {
            Tile::Window { id: root, .. } if *root == id => self.root = None,
            _ => {
                if let Some(parent) = root.find_parent(id) {
                    let survivor = if let Tile::Split { first, second, .. } = parent {
                        if matches!(**first, Tile::Window { id, .. } if id == id) {
                            std::mem::replace(&mut **second, Tile::Dummy)
                        } else {
                            std::mem::replace(&mut **first, Tile::Dummy)
                        }
                    } else {
                        unreachable!()
                    };

                    *parent = survivor;
                }

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
        if let Some(root) = &mut self.root
            && let Some(tile) = root.find(id)
            && matches!(tile, Tile::Window { .. })
        {
            *tile = Tile::Window { id, content };
        }
    }

    pub fn get_rect(&self, id: WindowId) -> Option<Rect> {
        if let Some(f) = self.floating.iter().find(|f| f.id == id) {
            return Some(f.rect);
        }

        if let Some(root) = &self.root {
            // FIXME: memoize this.
            let mut layout = Vec::new();
            root.layout(self.rect, &mut layout);

            return layout.into_iter().find(|(window, _)| *window == id).map(|(_, rect)| rect);
        }

        None
    }

    pub fn get_window(&self, pos: Pos) -> Option<WindowId> {
        for floating in self.floating.iter().rev() {
            if floating.rect.contains(pos) {
                return Some(floating.id);
            }
        }

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

    pub fn navigate(&self, direction: Direction) -> Option<WindowId> {
        let Some(active) = self.active_window else {
            return None;
        };
        let Some(root) = &self.root else {
            return None;
        };

        let mut layout = Vec::new();
        root.layout(self.rect, &mut layout);
        let Some(&(_, active_rect)) = layout.iter().find(|(id, _)| *id == active) else {
            return None;
        };

        let mut res = None;
        let mut min = usize::MAX;
        for (id, rect) in layout {
            if id == active {
                continue;
            }

            let (x, y) = rect.intersects(active_rect);
            let valid = match direction {
                Direction::Left => rect.pos.x < active_rect.pos.x && y,
                Direction::Right => rect.pos.x > active_rect.pos.x && y,
                Direction::Up => rect.pos.y < active_rect.pos.y && x,
                Direction::Down => rect.pos.y > active_rect.pos.y && x,
            };

            if valid {
                let dist = rect.distance(active_rect);
                if dist < min {
                    min = dist;
                    res = Some(id);
                }
            }
        }

        res
    }

    pub fn render(&self, state: &State, screen: &mut Screen) {
        if let Some(root) = &self.root {
            root.render(state, screen, self.rect);
        }

        for floating in &self.floating {
            floating.content.render(state, &mut Viewport::new(screen, floating.rect), floating.id);
        }
    }
}
