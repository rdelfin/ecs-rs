//! Entity Component System Library (ECS)
//!
//! For info about why an ECS may be beneficial, see some of these articles:
//!
//! - http://gameprogrammingpatterns.com/component.html
//! - http://t-machine.org/index.php/2007/09/03/entity-systems-are-the-future-of-mmog-development-part-1/
//! - http://www.gamedev.net/page/resources/_/technical/game-programming/understanding-component-entity-systems-r3013
//! - http://cowboyprogramming.com/2007/01/05/evolve-your-heirachy/
//!
//! There is a large variety of ways an ECS may work. This particular one is similar to
//! [Artemis](http://gamadu.com/artemis/).
//! Although this isn't a port to Rust, most functionality should be similar, and the
//! tutorials/manual there should be able to make up for the current lack of documentation [FIXME]
//!
//! Here's the basic structure:
//!
//! - An `Entity` is just an identifier. It contains no data or logic whatsoever.
//! - A `Component` is a piece of data (eg: Position, Velocity, Colour). While containing logic can
//! sometimes be useful, it's best practice to avoid it wherever possible.
//! - A `System` runs all the logic. Most of the time, it filters out entities based on their
//! components, and only runs it's logic on the entities it's interested in. These filters are
//! called `Aspect`s. Some systems ignore entities, and just apply logic to the world itself.
//! - An `Aspect` is a simple helper to filter entities based on their components.
//! - A `Manager` is simply an object that observes when an entity is
//! added/activated/deactivated/removed. They are used to help 'manage' the entities, rather than
//! define data or logic.
//! - The `World` organises all the above items together to make sure everything runs as it should.

#![crate_name = "ecs"]
#![crate_type = "lib"]

#![feature(box_syntax, core, collections, hash, std_misc)]

pub use aspect::Aspect;
pub use component::{Component, ComponentList};
pub use component::{EntityBuilder, EntityModifier};
pub use entity::{Entity, EntityIter};
pub use system::{System, Process};
pub use world::{ComponentManager, SystemManager, DataHelper, World};

use std::ops::{Deref};

pub mod aspect;
pub mod component;
pub mod entity;
pub mod system;
pub mod world;

pub struct BuildData<'a, T: ComponentManager>(&'a Entity);

impl<'a, T: ComponentManager> BuildData<'a, T>
{
    pub fn insert<U: Component>(&self, c: &mut ComponentList<U>, component: U) -> Option<U>
    {
        unsafe { c.insert(self.0, component) }
    }
}

pub struct ModifyData<'a, T: ComponentManager>(&'a Entity);

impl<'a, T: ComponentManager> ModifyData<'a, T>
{
    pub fn insert<U: Component>(&self, c: &mut ComponentList<U>, component: U) -> Option<U>
    {
        unsafe { c.insert(self.0, component) }
    }

    pub fn get<U: Component>(&self, c: &ComponentList<U>) -> Option<U> where U: Clone
    {
        unsafe { c.get(self.0) }
    }

    pub fn has<U: Component>(&self, c: &ComponentList<U>) -> bool
    {
        unsafe { c.has(self.0) }
    }

    pub fn remove<U: Component>(&self, c: &mut ComponentList<U>) -> Option<U>
    {
        unsafe { c.remove(self.0) }
    }
}

pub struct EntityData<'a, T: ComponentManager>(&'a Entity);

impl<'a, T: ComponentManager> Deref for EntityData<'a, T>
{
    type Target = Entity;
    fn deref(&self) -> &Entity
    {
        self.0
    }
}

impl<'a, T: ComponentManager> EntityData<'a, T>
{
    pub fn get<U: Component>(&self, c: &ComponentList<U>) -> Option<U> where U: Clone
    {
        unsafe { c.get(self.0) }
    }

    pub fn borrow<'b, U: Component>(&self, c: &'b mut ComponentList<U>) -> Option<&'b mut U>
    {
        unsafe { c.borrow(self.0) }
    }

    pub fn has<U: Component>(&self, c: &ComponentList<U>) -> bool
    {
        unsafe { c.has(self.0) }
    }
}

#[macro_use]
mod macros
{
    #[macro_export]
    macro_rules! components {
        {
            $Name:ident {
                $(#[$kind:ident] $field_name:ident : $field_ty:ty),+
            }
        } => {
            pub struct $Name {
                $(
                    $field_name : $crate::ComponentList<$field_ty>,
                )+
            }

            unsafe impl $crate::ComponentManager for $Name
            {
                unsafe fn new() -> $Name
                {
                    $Name {
                        $(
                            $field_name : $crate::ComponentList::$kind(),
                        )+
                    }
                }

                unsafe fn remove_all(&mut self, entity: &$crate::Entity)
                {
                    $(
                        self.$field_name.remove(entity);
                    )+
                }
            }
        };
    }

    #[macro_export]
    macro_rules! systems {
        {
            $Name:ident<$components:ty> {
                $($field_name:ident : $field_ty:ty = $field_init:expr),+
            }
        } => {
            pub struct $Name {
                $(
                    $field_name : $field_ty,
                )+
            }

            unsafe impl $crate::SystemManager<$components> for $Name
            {
                #[allow(unused_unsafe)] // The aspect macro is probably going to be used here and it also expands to an unsafe block.
                unsafe fn new() -> $Name
                {
                    $Name {
                        $(
                            $field_name : $field_init,
                        )+
                    }
                }

                unsafe fn activated(&mut self, en: $crate::EntityData<$components>, co: &$components)
                {
                    $(
                        self.$field_name.activated(&en, co);
                    )+
                }

                unsafe fn reactivated(&mut self, en: $crate::EntityData<$components>, co: &$components)
                {
                    $(
                        self.$field_name.reactivated(&en, co);
                    )+
                }

                unsafe fn deactivated(&mut self, en: $crate::EntityData<$components>, co: &$components)
                {
                    $(
                        self.$field_name.deactivated(&en, co);
                    )+
                }

                unsafe fn update(&mut self, co: &mut $crate::DataHelper<$components>)
                {
                    $(
                        if self.$field_name.is_active() {
                            self.$field_name.process(co);
                        }
                    )+
                }
            }
        };
    }

    #[macro_export]
    macro_rules! aspect {
        {
            <$components:ty>
            all: [$($all_field:ident),*]
            none: [$($none_field:ident),*]
        } => {
            unsafe {
                Aspect::new(box |en: &$crate::EntityData<$components>, co: &$components| {
                    ($(en.has(&co.$all_field) &&)* true) &&
                    !($(en.has(&co.$none_field) ||)* false)
                })
            }
        };
        {
            <$components:ty>
            all: [$($field:ident),*]
        } => {
            aspect!(
                <$components>
                all: [$($field),*]
                none: []
            )
        };
        {
            <$components:ty>
            none: [$($field:ident),*]
        } => {
            aspect!(
                <$components>
                all: []
                none: [$($field),*]
            )
        };
    }
}
