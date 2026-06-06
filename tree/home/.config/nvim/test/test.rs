use std::collections::HashMap;
use std::fmt::{self, Display};
use std::sync::Arc;

/// 常量
const MAX_SIZE: usize = 1024;
const GREETING: &str = "Hello, World!";

/// 枚举
#[derive(Debug, Clone)]
enum Status {
    Active,
    Inactive { reason: String },
    Pending(u64),
}

/// Trait 定义
trait Cacheable: Display + Send + Sync {
    fn cache_key(&self) -> String;
    fn ttl(&self) -> u64 {
        3600 // default TTL
    }
}

/// 泛型结构体
#[derive(Debug)]
struct Cache<T: Cacheable> {
    store: HashMap<String, Arc<T>>,
    capacity: usize,
    hit_count: u64,
}

impl<T: Cacheable> Cache<T> {
    fn new(capacity: usize) -> Self {
        Self {
            store: HashMap::with_capacity(capacity),
            capacity,
            hit_count: 0,
        }
    }

    fn get(&mut self, key: &str) -> Option<Arc<T>> {
        if let Some(value) = self.store.get(key) {
            self.hit_count += 1;
            return Some(Arc::clone(value));
        }
        None
    }

    fn insert(&mut self, item: T) -> Result<(), &'static str> {
        if self.store.len() >= self.capacity {
            return Err("Cache is full");
        }
        let key = item.cache_key();
        self.store.insert(key, Arc::new(item));
        Ok(())
    }
}

/// 实现 Display
impl<T: Cacheable> Display for Cache<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cache(size={}, cap={}, hits={})",
            self.store.len(),
            self.capacity,
            self.hit_count
        )
    }
}

/// 主函数
fn main() {
    let mut cache: Cache<User> = Cache::new(MAX_SIZE);
    let user = User {
        id: 42,
        name: String::from("Alice"),
        active: true,
    };

    match cache.insert(user) {
        Ok(()) => println!("{GREETING}"),
        Err(e) => eprintln!("Error: {e}"),
    }

    for i in 0..10u64 {
        let status = if i % 2 == 0 {
            Status::Active
        } else {
            Status::Pending(i)
        };
        println!("Item {i}: {status:?}");
    }
}

#[derive(Debug)]
struct User {
    id: u64,
    name: String,
    active: bool,
}

impl Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "User({}, {})", self.id, self.name)
    }
}

impl Cacheable for User {
    fn cache_key(&self) -> String {
        format!("user:{}", self.id)
    }
}
