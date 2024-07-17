use bevy::prelude::*;
use std::{
    ops::{Deref, DerefMut},
    ptr,
};

#[derive(Debug)]
pub struct UnsafeWorld(pub *mut World);

impl UnsafeWorld {
    pub const fn empty() -> Self {
        Self(ptr::null_mut())
    }
}

unsafe impl Send for UnsafeWorld {}
unsafe impl Sync for UnsafeWorld {}

impl Deref for UnsafeWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for UnsafeWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}
