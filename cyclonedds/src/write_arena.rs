use std::any::Any;

#[derive(Default)]
pub struct WriteArena {
    slots: Vec<Box<dyn Any>>,
}

impl WriteArena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hold<T: 'static>(&mut self, value: T) -> *const T {
        let boxed = Box::new(value);
        let ptr = (&*boxed) as *const T;
        self.slots.push(boxed);
        ptr
    }
}
