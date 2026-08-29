use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

/// An async safe wrapper around a `HashMap` to conveniently map types to
/// values.
pub struct TypeMap {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl TypeMap {
    pub fn new() -> Self { Self { map: HashMap::new() } }

    pub fn insert<T: 'static + Send + Sync>(&mut self, v: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(v));
    }

    pub fn remove<T: 'static + Send + Sync>(&mut self) -> Option<T> {
        self.map.remove(&TypeId::of::<T>()).and_then(|v| v.downcast::<T>().ok()).map(|v| *v)
    }

    pub fn contains<T: 'static + Send + Sync>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.map.get(&TypeId::of::<T>()).and_then(|v| v.downcast_ref::<T>())
    }

    pub fn get_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut T> {
        self.map.get_mut(&TypeId::of::<T>()).and_then(|v| v.downcast_mut::<T>())
    }
}
