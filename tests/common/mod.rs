use eonix::{Component, NoSend, Resource};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct C1(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct C2(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct C3(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub struct R1(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, NoSend)]
pub struct R2(pub u32);
