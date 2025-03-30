use eonix::Component;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct C1(pub u32);
impl Component for C1 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct C2(pub u32);
impl Component for C2 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct C3(pub u32);
impl Component for C3 {}
