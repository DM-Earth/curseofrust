use std::ops::{Deref, DerefMut};

use cacao::{
    foundation::id,
    objc::{msg_send, runtime::Object},
};

/// Swim through the objective sea to find a rusty old pal.
pub fn app_from_objc<T>() -> &'static mut T {
    unsafe {
        use cacao::foundation::load_or_register_class;
        let objc_app: id = msg_send![
            load_or_register_class("NSApplication", "RSTApplication", |_| {}),
            sharedApplication
        ];
        let objc_delegate: id = msg_send![objc_app, delegate];
        let rs_delegate_ptr: usize = *Object::ivar(&*objc_delegate, "rstAppPtr");
        &mut *(rs_delegate_ptr as *mut T)
    }
}

/// Avoid actually creating something before assigning to it.\
/// Not robust, but enough for private use.
pub struct OnceAssign<T>(pub Option<T>);

impl<T> Deref for OnceAssign<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl<T> DerefMut for OnceAssign<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}

impl<T> OnceAssign<T> {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn set(&mut self, content: T) {
        if self.0.is_none() {
            self.0 = Some(content);
        }
    }
}
