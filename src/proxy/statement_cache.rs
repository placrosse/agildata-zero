use std::sync::Mutex;
use std::rc::Rc;
use std::collections::HashMap;
use error::ZeroError;
use query::{Token};

use super::physical_planner::{PhysicalPlan};

pub struct StatementCache {
    cache: Mutex<HashMap<Vec<Token>, Rc<PhysicalPlan>>>
}

impl StatementCache {
    pub fn new() -> Self {
        StatementCache {
            cache: Mutex::new(HashMap::new())
        }
    }

    pub fn get(&self, key: &Vec<Token>) -> Option<Rc<PhysicalPlan>> {
        let data = self.cache.lock().unwrap();

        match data.get(key) {
            Some(rc) => Some(rc.clone()),
            None => None
        }
    }

    pub fn put(&self, key: Vec<Token>, ep: PhysicalPlan) -> Rc<PhysicalPlan> {
        let mut data = self.cache.lock().unwrap();
        let value = Rc::new(ep);
        let reference = value.clone();
        data.insert(key, value);
        reference
    }
}