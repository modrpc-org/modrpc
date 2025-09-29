pub struct InterfaceEvent<Ty> {
    pub name: &'static str,
    pub topic: u32,
    _ty: std::marker::PhantomData<Ty>,
}

impl<Ty> Clone for InterfaceEvent<Ty> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            topic: self.topic,
            _ty: std::marker::PhantomData,
        }
    }
}
impl<Ty> Copy for InterfaceEvent<Ty> {}

pub struct InterfaceBuilder {
    next_topic: u32,
}

impl InterfaceBuilder {
    pub fn new() -> InterfaceBuilder {
        InterfaceBuilder { next_topic: 0 }
    }

    pub fn event<Ty>(&mut self, name: &'static str) -> InterfaceEvent<Ty> {
        let topic = self.next_topic;
        self.next_topic += 1;

        InterfaceEvent {
            name,
            topic,
            _ty: std::marker::PhantomData,
        }
    }
}
