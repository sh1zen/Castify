use std::any::Any;
use std::sync::Arc;

pub struct AnyBox {
    data: Arc<Box<dyn Any + Send + Sync>>,
    //type_id: TypeId,
}

impl AnyBox
{
    pub fn new<T>(any: T) -> AnyBox
    where
        T: Any + Clone + Send + Sync,
    {
        AnyBox {
            data: Arc::new(Box::new(any.clone())),
            //type_id: TypeId::of::<T>(),
        }
    }

    pub fn downcast<U: Any>(&self) -> Option<&U> {
        self.data.downcast_ref::<U>()
    }

    /*pub fn downcast_mut<U: Any>(&self) -> Option<&mut U> {
        let mut fat_ptr = self.data.as_ref();

        let x =  unsafe { Box::from_raw(fat_ptr as *mut U) };

        let arc = Arc::from(&**self.data);
        let ptr = Arc::as_ref(&self.data);
        ptr.downcast_ref::<U>()
    }*/
}

impl Clone for AnyBox
{
    fn clone(&self) -> Self {
        AnyBox {
            data: Arc::clone(&self.data)
        }
    }
}

impl std::fmt::Debug for AnyBox {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{:?}", self.data.type_id())
    }
}
