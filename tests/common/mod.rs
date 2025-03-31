use eonix::{Component, NoSend, Resource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct C1(pub u32);
impl Component for C1 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct C2(pub u32);
impl Component for C2 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct C3(pub u32);
impl Component for C3 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct R1(pub u32);
impl Resource for R1 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct R2(pub u32);
impl NoSend for R2 {}
