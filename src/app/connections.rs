use super::Id;

#[derive(Default)]
pub struct Connections {
    next_id: Id,
}

impl Connections {
    pub fn allocate_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        return id;
    }
}
