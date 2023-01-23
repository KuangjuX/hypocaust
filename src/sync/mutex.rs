pub trait Mutex: Sync + Send {
    fn lock(&self);
    fn unlock(&self);
}